//! Networking layer (like UDP, not the "blockchain network").
//!
//! This module is responsible for handling the network communication layer.
//! To limit complexity and focus on the core concepts, we simplify the network to be connection-less (UDP-like).
//! We also assume that the network is secure.
//!
//! We introduce the following concepts:
//! 1. `Node`: Single node in the network. It can have multiple connections.
//! 2. `Hop`: A hop is one step between two peers. It can introduce various types of delays and errors.
//! 4. `Message`: A message is a packet of data that is sent between two peers. It can be of various types.
//! 5. `Header`: A header is a part of the message that contains metadata about the message, required by the network layer.
//! 6. `Router`: Special type of `Hop` that is an entrypoint to the network, responsible for routing messages between peers.
//! 7. `Network`: A collection of nodes and routers that form a network.
//!
//! Real life example is that some node constructs a message and sends it to the router. The router then
//! processes the message using various hops and finally sends it to the recipient node. The recipient node
//! then processes the message.

use std::sync::Arc;

mod basic_router;
mod builder;
mod delay_hop;
mod network;

#[allow(unused_imports)]
pub use {basic_router::BasicRouter, builder::NetworkBuilder, delay_hop::DelayHop};

use futures::future::BoxFuture;
pub use network::Network;
use tokio::sync::mpsc;

pub type NodeID = u64;

#[derive(Clone, Debug)]
pub struct Header {
    pub version: u8,
    pub sender: NodeID,
    pub recipient: NodeID,

    pub timestamp: tokio::time::Instant,
}

/// Message that can be sent between nodes.
///
/// Contains some payload of type `P`.
///
/// Note: only `Vec<u8>` payload is supported by the router at the moment.
/// See [Router] for more details.
#[derive(Clone)]
pub struct Message<P: Clone> {
    pub header: Header,
    pub payload: P,
}

/// A hop is a single step between two peers.
///
/// It can introduce various types of delays and errors.
/// Note it cannot see message contents, which is considered secure.
pub trait Hop: Send + Sync {
    /// Process the message when it goes through the hop.
    ///
    /// ## Returns
    ///
    /// If the message should be dropped, return `None`.
    ///
    // TODO: Refactor this fn to be async to avoid using BoxFuture
    fn process<'a>(&'a self, header: &Header) -> BoxFuture<'a, Option<()>>;
}

/// Network node that can send and receive messages.
///
/// Each node needs to subscribe to the router using [Router::subscribe] and send messages using [Router::send].
///
/// Note each node should use the same, network-wide clock.
#[async_trait::async_trait]
pub trait Node<R: Router>: Send + Sync {
    /// Start the node.
    ///
    /// This function is called by the [Network::start](crate::networking::Network::start) when the node is started.
    /// The node should subscribe to the router with [Router::subscribe] and start its logic (eg. sending messages).
    async fn start(&self);

    /// Change router for the node.
    ///
    /// Usually called by the network builder.
    async fn set_router(&self, router: Arc<R>);

    /// Return node ID.
    fn id(&self) -> NodeID;
}

/// Router that routes messages between nodes.
///
/// Nodes send messages to the router using [Router::send] and subscribe to messages using [Router::subscribe].
#[async_trait::async_trait]
pub trait Router: Send + Sync {
    /// Send a message to the network.
    ///
    /// The router will process the message using various hops and send it to the recipient.
    /// If the recipient is not available, the message will be dropped.
    async fn send(&self, message: Message<Vec<u8>>);

    /// Subscribe to messages for the given node ID.
    ///
    /// Returns a receiver that will receive messages for the given node ID.
    /// The receiver will be closed when the router is gone (most likely during shutdown).
    ///
    /// [Node] should subscribe during startup, to receive messages from the router.
    async fn subscribe(&self, id: NodeID) -> mpsc::Receiver<Message<Vec<u8>>>;

    /// Add a hop to the router.
    ///
    /// The router will process the message using the hops in the order they were added.
    /// If the hop returns `None`, the message will be dropped.
    async fn add_hop(&self, hop: Arc<dyn Hop>);
}
