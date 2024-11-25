//! Simple node that just responds to messages.

use crate::{
    networking::{Header, Message, Node, NodeID, Router},
    time::{Clock, TokioClock},
};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Notify};

pub struct PingNode<R: Router> {
    /// Identifier of the node.
    id: NodeID,
    /// List of peers of the node to which it sends messages
    peers: Vec<NodeID>,
    /// Router used by this node
    router: Mutex<Arc<R>>,
    /// Global clock used by the node
    clock: TokioClock,
    /// Delay between pings (simulated time, not real system time). Defaults to 0.
    delay: tokio::time::Duration,

    /// Signal completion
    pub done: Arc<Notify>,
}

impl<R: Router> PingNode<R> {
    /// Create a new node with the given ID, router, and list of peers.
    /// The node will use the given clock.
    pub fn new(id: NodeID, router: Arc<R>, peers: Vec<NodeID>, clock: TokioClock) -> Self {
        PingNode {
            id,
            router: Mutex::new(router),
            peers,
            clock,
            delay: tokio::time::Duration::from_secs(0),
            done: Arc::new(Notify::new()),
        }
    }

    /// Set the delay between pings.
    pub fn set_delay(&mut self, delay: tokio::time::Duration) {
        self.delay = delay;
    }

    /// Receive messages from the router.
    ///
    /// This is a worker that is started in its own thread by the [`PingNode::start`] method.
    async fn receive(my_id: NodeID, mut rx: mpsc::Receiver<PingMessage>, clock: TokioClock) {
        let start = clock.start_time();
        while let Some(message) = rx.recv().await {
            let now = clock.now();
            println!(
                "{:?}: Node {} received message from {}, send time {:?}",
                now.duration_since(start).as_millis(),
                my_id,
                message.header.sender,
                message.header.timestamp.duration_since(start).as_millis(),
            );
        }
    }

    /// Send a message to all peers.
    async fn broadcast(&self, body: &str) {
        for peer in self.peers.iter() {
            let message = PingMessage {
                header: Header {
                    version: 1,
                    sender: self.id,
                    recipient: *peer,
                    timestamp: self.clock.now(),
                },
                payload: body.as_bytes().to_vec(),
            };
            self.router.lock().await.send(message).await;
        }
    }
}

#[async_trait::async_trait]
impl<R: Router> Node<R> for PingNode<R> {
    fn id(&self) -> NodeID {
        self.id
    }

    async fn set_router(&self, router: Arc<R>) {
        *self.router.lock().await = router;
    }

    async fn start(&self) {
        let rx = self.router.lock().await.subscribe(self.id).await;
        let id = self.id;
        let done = self.done.clone();
        let clock = self.clock.clone();
        let recv_abort = tokio::spawn(async move {
            Self::receive(id, rx, clock).await;
            done.notify_one()
        })
        .abort_handle();

        // we send a few pings first, then wait for the responses
        for i in 0..5 {
            self.broadcast(&format!("ping {}", i)).await;
            self.clock.sleep(self.delay).await;
        }

        // wait for completion
        self.done.notified().await;
        drop(recv_abort); // dropping explicitly to avoid warnings
    }
}

/// Ping message contains some bytes as payload (only bytes are supported by the router atm).
type PingMessage = Message<Vec<u8>>;

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use tokio::time::timeout;

    use crate::{
        networking::{BasicRouter, DelayHop, NetworkBuilder, NodeID, Router},
        time::TokioClock,
    };

    use super::PingNode;

    #[tokio::test]
    async fn test_ping_node() {
        const NUM_NODES: usize = 5;

        // Create a clock; only one instance allowed system-wide (will panic otherwise)
        let clock = TokioClock::new();

        // Create a router with a single hop that will delay and sometimes drop messages
        let router = Arc::new(BasicRouter::default());
        let hop = Arc::new(DelayHop::new(clock.clone()));
        router.add_hop(hop).await;

        // Initialize builder with a router and clock
        let mut builder = NetworkBuilder::new()
            .with_router(router.clone())
            .with_clock(clock.clone());

        // Create nodes and add them to the builder
        for id in 0..NUM_NODES as NodeID {
            // All nodes except me are my peers
            let peers = (0..5 as NodeID).filter(|&x| x != id).collect();

            let mut node = PingNode::new(id, Arc::clone(&router), peers, clock.clone());
            // Each node has a different delay
            node.set_delay(tokio::time::Duration::from_millis(id * 100));
            builder = builder.add_node(Arc::new(node));
        }

        // Finally, build the network
        let mut network = builder.build().await.expect("failed to build network");

        // Start the network and kill it after 60 seconds; note this is "test time", not real time.
        timeout(Duration::from_secs(60), network.start()).await.ok();
    }
}
