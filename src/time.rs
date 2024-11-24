//! Implementation of network-wide time.
//! This module provides an abstract time implementation that can be used to
//! simulate time in a deterministic way. This is useful for testing and
//! debugging purposes.
//!
//! The `Clock` trait provides an interface for getting the current time and
//! advancing the time by a specified amount. The `SimulatedClock` struct
//! implements this trait using a simulated clock that can be controlled by the
//! user.
//!

use std::{collections::BTreeSet, sync::Arc, task::Wake};

use tokio::{
    sync::{Mutex, Notify},
    time::{advance, Instant},
};

pub trait Clock {
    type Timestamp;
    type Duration;

    /// Get the current time.
    fn now(&self) -> Self::Timestamp;

    /// Sleep for a given amount of time.
    async fn sleep(&self, duration: Self::Duration);
}

// /// Simulated clock implementation.
// /// This struct provides a simulated clock that can be controlled by the user.
// #[derive(Default)]
// pub struct SimulatedClock {
//     time: AtomicU64,
// }

// impl Clock for SimulatedClock {
//     type Timestamp = u64;
//     type Duration = u64;
//     fn now(&self) -> Self::Timestamp {
//         self.time.load(std::sync::atomic::Ordering::Relaxed)
//     }

//     async fn sleep(&mut self, duration: Self::Duration) {
//         self.time
//             .fetch_add(duration, std::sync::atomic::Ordering::Relaxed);
//     }
// }

#[derive(Clone)]
pub struct TokioClock {
    /// queue of scheduled wakeups
    queue: Arc<Mutex<BTreeSet<Instant>>>,
    /// notify the clock to advance to next scheduled wakeup
    advance: Arc<Notify>,

    /// Start time, when the clock was created
    start_time: Instant,
}

impl TokioClock {
    pub fn new() -> Self {
        tokio::time::pause();
        let me = Self {
            queue: Default::default(),
            advance: Arc::new(Notify::new()),
            start_time: tokio::time::Instant::now(),
        };

        let wakeup = me.advance.clone();
        let queue = me.queue.clone();
        tokio::spawn(Self::process_queue(wakeup, queue));

        me
    }

    /// Get start time of the clock.
    pub fn start_time(&self) -> Instant {
        self.start_time
    }

    /// Advance the clock to the next scheduled wakeup.
    ///
    /// This function will sleep until notified with `wakeup`. Once notified, it will advance the clock
    /// to the next scheduled wakeup.
    async fn process_queue(wakeup: Arc<Notify>, queue: Arc<Mutex<BTreeSet<Instant>>>) {
        loop {
            wakeup.notified().await;
            let mut guard = queue.lock().await;
            if let Some(next_instant) = guard.pop_first() {
                let now = tokio::time::Instant::now();
                if next_instant > now {
                    advance(next_instant - now).await;
                }
            }
        }
    }

    /// Enqueue a wakeup at the given time.
    async fn enqueue(&self, when: Instant) {
        self.queue.lock().await.insert(when);
        // Ensure other threads have their chance to run
        tokio::task::yield_now().await;
        // trigger the clock to advance
        self.advance.notify_one();
    }
}

impl Clock for TokioClock {
    type Timestamp = tokio::time::Instant;
    type Duration = tokio::time::Duration;

    fn now(&self) -> Self::Timestamp {
        tokio::time::Instant::now()
    }

    async fn sleep(&self, duration: Self::Duration) {
        let until = self.now() + duration;
        self.enqueue(until).await;
        tokio::time::sleep_until(until).await;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::Clock;
    use tokio::{sync::Mutex, time::Duration};
    /// Given a TokioClock and 3 threads,
    /// when each thread sleeps for different amount of time,
    /// then ordering of execution is correct.
    #[tokio::test]
    async fn test_tokio_clock() {
        let order = Arc::new(Mutex::new(Vec::<usize>::new()));
        let clock = super::TokioClock::new();
        let start = clock.now();

        let t1_clock = clock.clone();
        let t1_order = order.clone();
        let t1 = tokio::spawn(async move {
            t1_clock.clone().sleep(Duration::from_millis(100)).await;
            t1_order.lock().await.push(1);
        });

        let t2_clock = clock.clone();
        let t2_order = order.clone();
        let t2 = tokio::spawn(async move {
            t2_clock.sleep(Duration::from_millis(200)).await;
            t2_order.lock().await.push(2);
        });

        let t3_clock = clock.clone();
        let t3_order = order.clone();
        let t3 = tokio::spawn(async move {
            t3_clock.sleep(Duration::from_millis(300)).await;
            t3_order.lock().await.push(3);
        });

        tokio::try_join!(t1, t2, t3).unwrap();
        println!("Order: {:?}", order.lock().await);
        assert_eq!(order.lock().await.as_slice(), &[1, 2, 3]);
    }

    #[tokio::test]
    async fn paused_time() {
        tokio::time::pause();
        let start = std::time::Instant::now();
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        println!("{:?}ms", start.elapsed().as_millis());
        tokio::time::resume();
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        println!("{:?}ms", start.elapsed().as_millis());
    }
}
