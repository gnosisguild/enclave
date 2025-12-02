// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::simulate_libp2p_net;
use anyhow::*;
use e3_ciphernode_builder::CiphernodeHandle;
use e3_events::Event;
use e3_events::{EnclaveEvent, GetEvents, ResetHistory, TakeEvents};
use std::{future::Future, ops::Deref, pin::Pin, time::Duration};
use tokio::time::timeout;

// This type allows us to store various dynamic async callbacks
type SetupFn<'a> =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<CiphernodeHandle>> + 'a>> + 'a>;
type ThenFn<'a> =
    Box<dyn Fn(CiphernodeHandle) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> + 'a>;

/// This builds a ciphernode system using the actor model only. This helps us simulate the network
/// in tests that we can run in the /crates/tests crate
/// ```ignore
/// let nodes = CiphernodeSystemBuilder::new()
///     .add_group(6, || async {
///         setup_local_ciphernode(bus, rng, true, rand_eth_addr(), None, cipher).await
///     })
///     .add_group(1, || async {
///         setup_aggregator_ciphernode(bus, rng, true, rand_eth_addr(), None, cipher).await
///     })
///     .build()
///     .await?;
/// ```
pub struct CiphernodeSystemBuilder<'a> {
    // Various groups with different setup functions
    groups: Vec<(u32, SetupFn<'a>)>,
    thens: Vec<ThenFn<'a>>,
    simulate: bool,
}

impl<'a> CiphernodeSystemBuilder<'a> {
    pub fn new() -> Self {
        Self {
            groups: Vec::new(),
            thens: Vec::new(),
            simulate: false,
        }
    }

    /// Add a group of nodes with a specific configuration to the ciphernode system.
    /// We can add multiple groups of nodes each with a different configuration and count.
    pub fn add_group<F, Fut>(mut self, count: u32, setup_fn: F) -> Self
    where
        F: Fn() -> Fut + 'a,
        Fut: Future<Output = Result<CiphernodeHandle>> + 'a,
    {
        let wrapped_fn = Box::new(
            move || -> Pin<Box<dyn Future<Output = Result<CiphernodeHandle>> + 'a>> {
                Box::pin(setup_fn())
            },
        );
        self.groups.push((count, wrapped_fn));
        self
    }

    /// Add gossip simulation. This takes all event bus events on local ciphernode busses that have been set for
    /// broadcast and broadcasts them to all other nodes.
    pub fn simulate_libp2p(mut self) -> Self {
        self.simulate = true;
        self
    }

    /// Build the system returning a list of all nodes
    pub async fn build(self) -> Result<CiphernodeSystem> {
        let mut nodes = Vec::new();

        for (count, setup_fn) in self.groups {
            for _ in 0..count {
                nodes.push(setup_fn().await?);
            }
        }

        if self.simulate {
            simulate_libp2p_net(&nodes);
        }

        for then_fn in self.thens {
            for node in nodes.clone() {
                then_fn(node).await?;
            }
        }

        Ok(CiphernodeSystem(nodes))
    }
}

pub struct CiphernodeSystem(Vec<CiphernodeHandle>);

impl CiphernodeSystem {
    pub async fn get_history(&self, index: usize) -> Result<CiphernodeHistory> {
        let Some(node) = self.0.get(index) else {
            return Ok(CiphernodeHistory(vec![]));
        };

        let history = if let Some(history) = node.history() {
            history.send(GetEvents::new()).await?
        } else {
            vec![]
        };

        Ok(CiphernodeHistory(history))
    }

    pub async fn take_history(&self, index: usize, count: usize) -> Result<CiphernodeHistory> {
        self.take_history_with_timeout(index, count, Duration::from_millis(4000))
            .await
    }

    pub async fn take_history_with_timeout(
        &self,
        index: usize,
        count: usize,
        tout: Duration,
    ) -> Result<CiphernodeHistory> {
        let Some(node) = self.0.get(index) else {
            bail!("No node found");
        };

        let Some(history) = node.history() else {
            return Ok(CiphernodeHistory(vec![]));
        };

        let history = timeout(tout, history.send(TakeEvents::new(count)))
            .await
            .context(format!(
                "Could not take {} events from node {}",
                count, index
            ))??;

        Ok(CiphernodeHistory(history))
    }
    pub async fn flush_all_history(&self, millis: u64) -> Result<()> {
        let nodes = self.0.clone();
        for node in nodes.iter() {
            let Some(history) = node.history() else {
                break;
            };
            loop {
                let nhs = history.send(TakeEvents::new(1));
                let tr = timeout(Duration::from_millis(millis), nhs).await;
                if !tr.is_ok() {
                    break;
                }
            }
            history.send(ResetHistory).await?;
        }

        Ok(())
    }
}

impl Deref for CiphernodeSystem {
    type Target = Vec<CiphernodeHandle>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct CiphernodeHistory(Vec<EnclaveEvent>);

impl CiphernodeHistory {
    pub fn filter_by_event_type(&self, event_type: String) -> Vec<EnclaveEvent> {
        self.0
            .iter()
            .filter(|e| e.event_type() == event_type)
            .cloned()
            .collect()
    }

    pub fn event_types(&self) -> Vec<String> {
        self.0.iter().map(|e| e.event_type()).collect()
    }
}

impl Deref for CiphernodeHistory {
    type Target = Vec<EnclaveEvent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix::prelude::*;
    use e3_data::InMemStore;
    use e3_events::{hlc::Hlc, BusHandle, EventBus, EventBusConfig};

    async fn mock_setup_node(address: String) -> Result<CiphernodeHandle> {
        // Create mock actors for the test
        let store = InMemStore::new(true).start();
        let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
        let history = EventBus::<EnclaveEvent>::history(&bus);
        let errors = EventBus::<EnclaveEvent>::error(&bus);
        let bus = BusHandle::new(bus);

        Ok(CiphernodeHandle {
            address,
            store: (&store).into(),
            bus,
            history: Some(history),
            errors: Some(errors),
        })
    }

    #[actix::test]
    async fn test_builder_creates_multiple_groups() {
        let nodes = CiphernodeSystemBuilder::new()
            .add_group(2, || mock_setup_node("node_a".to_string()))
            .add_group(3, || mock_setup_node("node_b".to_string()))
            .build()
            .await
            .expect("Should create nodes successfully");

        // Should have created 5 total nodes (2 + 3)
        assert_eq!(nodes.len(), 5);

        // Verify node addresses
        assert_eq!(nodes[0].address, "node_a");
        assert_eq!(nodes[1].address, "node_a");
        assert_eq!(nodes[2].address, "node_b");
        assert_eq!(nodes[3].address, "node_b");
        assert_eq!(nodes[4].address, "node_b");
    }
}
