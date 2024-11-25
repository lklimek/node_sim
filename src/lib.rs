#![allow(dead_code)]
//! Simple simulation of a network of nodes.
//!
//! This crate provides a simple simulation of a network of nodes that can send messages to each other.
//!
//! First, you need to implement the [Node](networking::Node) trait for your node type. See [PingNode](ping_node::PingNode) for an example.
//! Then you need to build your network with the [NetworkBuilder](networking::NetworkBuilder)
//! and start it with [Network::start](networking::Network::start).
//!
//! Your network will use a [Router](networking::Router) to route messages between nodes (like [networking::BasicRouter]),
//! and system clock (like [time::TokioClock]) to keep track of time.
//!
//! See tests in the [ping_node] module for examples.
//!
//! You can also introduce network delays and errors by implementing [Hop](networking::Hop) interface.
//! See [DelayHop](networking::DelayHop) for an example.
pub mod error;
pub mod networking;
pub mod ping_node;
pub mod time;
