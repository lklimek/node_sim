//! This module contains the implementation of the DelayHop struct.
//! The DelayHop struct is a hop that introduces a delay in the network.

use std::sync::atomic::AtomicU64;

use futures::{future::BoxFuture, FutureExt};

use crate::time::{Clock, TokioClock};

use super::Hop;

/// A DelayHop that introduces some variable, deterministic delay in the network.
///
/// If the calculated delay happens to be 0, the message is dropped.
pub struct DelayHop {
    clock: TokioClock,
    last_delay: AtomicU64,
}

impl DelayHop {
    /// Create a new DelayHop.
    pub fn new(clock: TokioClock) -> Self {
        Self {
            clock,
            last_delay: AtomicU64::new(10),
        }
    }
}
impl Hop for DelayHop {
    fn process<'a>(&'a self, header: &super::Header) -> BoxFuture<'a, Option<()>> {
        // Introduce a random delay
        let delay = self
            .last_delay
            .fetch_add(10, std::sync::atomic::Ordering::SeqCst)
            % 1000;
        if delay == 0 {
            return Box::pin(async { None });
        }

        println!(
            "Delaying message from {} to {} for {} ms",
            header.sender, header.recipient, delay
        );

        Box::pin(
            self.clock
                .sleep(std::time::Duration::from_millis(delay).into())
                .map(|_| Some(())),
        )
    }
}
