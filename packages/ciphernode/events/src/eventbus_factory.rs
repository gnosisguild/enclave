use actix::Actor;
use actix::Addr;
use once_cell::sync::Lazy;
use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::EnclaveEvent;
use crate::Event;
use crate::EventBus;
use crate::EventBusConfig;

// The singleton factory using once_cell
pub struct EventBusFactory {
    instances: Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl EventBusFactory {
    // Get the singleton instance of the factory
    pub fn instance() -> &'static EventBusFactory {
        static INSTANCE: Lazy<EventBusFactory> = Lazy::new(|| EventBusFactory {
            instances: Mutex::new(HashMap::new()),
        });

        &INSTANCE
    }

    // Get or create a singleton EventBus for the specific event type
    pub fn get_event_bus<E: Event>(&self, config: EventBusConfig) -> Addr<EventBus<E>> {
        let type_id = TypeId::of::<E>();
        let mut instances = self.instances.lock().unwrap();

        // If we already have this type of EventBus, return it
        if let Some(instance) = instances.get(&type_id) {
            return instance
                .downcast_ref::<Addr<EventBus<E>>>()
                .expect("Type mismatch in EventBusFactory")
                .clone();
        }

        // Create a new EventBus for this event type
        let event_bus = EventBus::<E>::new(config).start();

        // Store it in our HashMap
        instances.insert(type_id, Box::new(event_bus.clone()));

        event_bus
    }
}

pub fn get_enclave_event_bus() -> Addr<EventBus<EnclaveEvent>> {
    EventBusFactory::instance().get_event_bus::<EnclaveEvent>(EventBusConfig {
        deduplicate: true,
        capture_history: true,
    })
}
