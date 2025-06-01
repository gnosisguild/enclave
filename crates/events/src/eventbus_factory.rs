use actix::Actor;
use actix::Addr;
use once_cell::sync::Lazy;
use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::EnclaveEvent;
use crate::ErrorCollector;
use crate::ErrorEvent;
use crate::Event;
use crate::EventBus;
use crate::Subscribe;

// The singleton factory using once_cell
pub struct EventBusFactory {
    event_bus_cache: Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
    error_collector_cache: Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl EventBusFactory {
    // Get the singleton instance of the factory
    pub fn instance() -> &'static EventBusFactory {
        static INSTANCE: Lazy<EventBusFactory> = Lazy::new(|| EventBusFactory {
            event_bus_cache: Mutex::new(HashMap::new()),
            error_collector_cache: Mutex::new(HashMap::new()),
        });

        &INSTANCE
    }

    // Get or create a singleton EventBus for the specific event type
    pub fn get_event_bus<E: Event>(&self) -> Addr<EventBus<E>> {
        let type_id = TypeId::of::<E>();
        let mut event_bus_cache = self
            .event_bus_cache
            .lock()
            .expect("event_bus_cache mutex failed to lock");

        // If we already have this type of EventBus, return it
        if let Some(instance) = event_bus_cache.get(&type_id) {
            return instance
                .downcast_ref::<Addr<EventBus<E>>>()
                .expect("Type mismatch in EventBusFactory")
                .clone();
        }

        // Create a new EventBus for this event type
        let event_bus = EventBus::<E>::default().start();

        // Store it in our HashMap
        event_bus_cache.insert(type_id, Box::new(event_bus.clone()));

        event_bus
    }
    pub fn get_error_collector<E: ErrorEvent>(&self) -> Addr<ErrorCollector<E>> {
        let type_id = TypeId::of::<E>();
        let mut error_collector_cache = self.error_collector_cache.lock().unwrap();

        // If we already have this type of ErrorCollector, return it
        if let Some(instance) = error_collector_cache.get(&type_id) {
            return instance
                .downcast_ref::<Addr<ErrorCollector<E>>>()
                .expect("Type mismatch in EventBusFactory")
                .clone();
        }

        // Create a new EventBus for this event type
        let error_collector = ErrorCollector::<E>::new().start();
        // Importantly subscribe to events
        let bus = self.get_event_bus::<E>();
        bus.do_send(Subscribe::new("*", error_collector.clone().recipient()));
        // Store it in our HashMap
        error_collector_cache.insert(type_id, Box::new(error_collector.clone()));

        error_collector
    }
}

pub fn get_enclave_event_bus() -> Addr<EventBus<EnclaveEvent>> {
    EventBusFactory::instance().get_event_bus()
}

pub fn get_error_collector() -> Addr<ErrorCollector<EnclaveEvent>> {
    EventBusFactory::instance().get_error_collector()
}
