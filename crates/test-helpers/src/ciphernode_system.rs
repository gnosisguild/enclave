use actix::Addr;
use anyhow::*;
use e3_data::InMemStore;
use e3_events::{EnclaveEvent, ErrorCollector, EventBus, HistoryCollector};
use std::{future::Future, pin::Pin};

use crate::simulate_libp2p_net;

type SetupFn<'a> =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<CiphernodeSimulated>> + 'a>> + 'a>;

/// This builds a ciphernode system using the actor model only. This helps us simulate the network
/// in tests that we can run in the /crates/tests crate
pub struct CiphernodeSystemBuilder<'a> {
    // Various groups with different setup functions
    groups: Vec<(u32, SetupFn<'a>)>,
}

impl<'a> CiphernodeSystemBuilder<'a> {
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    pub fn add_group<F, Fut>(mut self, count: u32, setup_fn: F) -> Self
    where
        F: Fn() -> Fut + 'a,
        Fut: Future<Output = Result<CiphernodeSimulated>> + 'a,
    {
        let wrapped_fn = Box::new(
            move || -> Pin<Box<dyn Future<Output = Result<CiphernodeSimulated>> + 'a>> {
                Box::pin(setup_fn())
            },
        );
        self.groups.push((count, wrapped_fn));
        self
    }

    pub async fn build(self) -> Result<Vec<CiphernodeSimulated>> {
        let mut nodes = Vec::new();

        for (count, setup_fn) in self.groups {
            for _ in 0..count {
                nodes.push(setup_fn().await?);
            }
        }

        simulate_libp2p_net(&nodes);
        Ok(nodes)
    }
}

pub struct CiphernodeSimulated {
    pub address: String,
    pub store: Addr<InMemStore>,
    pub bus: Addr<EventBus<EnclaveEvent>>,
    pub history: Addr<HistoryCollector<EnclaveEvent>>,
    pub errors: Addr<ErrorCollector<EnclaveEvent>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix::prelude::*;
    use e3_events::EventBusConfig;

    async fn mock_setup_node(address: String) -> Result<CiphernodeSimulated> {
        // Create mock actors for the test
        let store = InMemStore::new(true).start();
        let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
        let history = EventBus::<EnclaveEvent>::history(&bus);
        let errors = EventBus::<EnclaveEvent>::error(&bus);

        Ok(CiphernodeSimulated {
            address,
            store,
            bus,
            history,
            errors,
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
