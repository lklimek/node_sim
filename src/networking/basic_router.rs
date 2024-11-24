//! Basic router implementation.

use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
use tokio::sync::{mpsc, Mutex};

use super::{Message, Node, NodeID, Router};

#[derive(Default)]
pub struct BasicRouter {
    /// Hops that introduce delays, errors, etc.
    hops: Mutex<Vec<Arc<dyn super::Hop>>>,

    /// Message channels for each subscribed node.
    subscriptions: Mutex<BTreeMap<NodeID, mpsc::Sender<Message<Vec<u8>>>>>,
}

#[async_trait::async_trait]
impl Router for BasicRouter {
    async fn subscribe(&self, id: NodeID) -> mpsc::Receiver<Message<Vec<u8>>> {
        let (tx, rx) = mpsc::channel(100);
        let mut guard = self.subscriptions.lock().await;
        guard.insert(id, tx);

        rx
    }

    async fn send(&self, message: Message<Vec<u8>>) {
        let from = message.header.sender;
        let to = message.header.recipient;

        println!("Sending message from {} to {}", from, to);

        let hops = self.hops.lock().await.clone();
        for hop in hops.iter() {
            hop.process(&message.header);
        }

        let mut guard = self.subscriptions.lock().await;
        let recipient = match guard.get(&to) {
            Some(tx) => tx,
            None => {
                println!(
                    "Recipient {} is gone, dropping message because we are UDP :)",
                    to
                );
                return;
            }
        };

        if let Err(e) = recipient.send(message).await {
            // Send failed, recipient is gone.
            println!("Send from {} to {} failed: {}", from, to, e);
            guard
                .remove(&to)
                .expect("recipient must exist, we just read it");
        }
        drop(guard);
    }

    async fn add_hop(&self, hop: Arc<dyn super::Hop>) {
        self.hops.lock().await.push(hop);
    }
}
