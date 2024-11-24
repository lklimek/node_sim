use std::sync::Arc;

use tokio::task::JoinSet;

use crate::time::TokioClock;

use super::{Node, Router};

pub struct Network<R: Router, N: Node<R>> {
    pub(super) clock: TokioClock, // TODO: abstract if needed
    pub(super) nodes: Vec<Arc<N>>,
    pub(super) router: Arc<R>,
    pub(super) node_handles: JoinSet<()>,
}

impl<R: Router, N: Node<R> + 'static> Network<R, N> {
    /// Start the network.
    ///
    /// This will start all nodes in the network and block until they are all done.
    ///
    /// You can cancel the network using the `cancel` method on the returned future.
    pub async fn start(&mut self) {
        let mut group = JoinSet::new();

        for node in self.nodes.iter() {
            let node = Arc::clone(node);
            group.spawn(async move { node.start().await });
        }

        group.join_all().await;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{networking::NetworkBuilder, ping_node::PingNode, time::TokioClock};

    #[tokio::test]
    async fn test_network_builder() {
        let clock = crate::time::TokioClock::new();
        let router = Arc::new(crate::networking::BasicRouter::default());
        let node = Arc::new(PingNode::new(0, router.clone(), vec![], clock.clone()));

        let network: crate::networking::Network<crate::networking::BasicRouter, _> =
            NetworkBuilder::new()
                .with_router(router)
                .add_node(node)
                .with_clock(clock)
                .build()
                .await;

        assert_eq!(network.nodes.len(), 1);
    }
}
