//! Network builder that allows creation of a complete network.

use std::sync::Arc;

use crate::{error::Error, time::TokioClock};

use super::{Network, Node, Router};

/// Network builder to construct a complete network.
/// Designed for chaining method calls.
pub struct NetworkBuilder<R: Router, N: Node<R>> {
    nodes: Vec<Arc<N>>,
    router: Option<Arc<R>>,
    clock: Option<TokioClock>, // TODO: abstract if needed
}

impl<R: Router, N: Node<R>> NetworkBuilder<R, N> {
    /// Create a new network builder.
    pub fn new() -> Self {
        NetworkBuilder {
            clock: None,
            nodes: Vec::new(),
            router: None,
        }
    }

    /// Add a node to the network.
    pub fn add_node(mut self, node: Arc<N>) -> Self {
        self.nodes.push(node);
        self
    }

    /// Set the router for the network.
    ///
    /// Note that nodes will be added to the router automatically during [build](NetworkBuilder::build).
    pub fn with_router(mut self, router: Arc<R>) -> Self {
        self.router = Some(router);
        self
    }

    /// Set the clock for the network.
    ///
    /// This is the clock that all nodes in the network will use.
    pub fn with_clock(mut self, clock: TokioClock) -> Self {
        self.clock = Some(clock);
        self
    }

    /// Build the network.
    ///
    /// You should start the network after building it with [Network::start()].
    pub async fn build(self) -> Result<Network<R, N>, Error> {
        let router = self
            .router
            .ok_or(Error::BuilderConfig("router must be set".to_string()))?;

        let clock = self
            .clock
            .ok_or(Error::BuilderConfig("clock must be set".to_string()))?;

        for node in self.nodes.iter() {
            node.set_router(Arc::clone(&router)).await;
        }

        Ok(Network {
            clock,
            nodes: self.nodes,
            router,
            node_handles: tokio::task::JoinSet::new(),
        })
    }
}
