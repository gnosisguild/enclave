// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Actor;
use actix::Addr;
use e3_events::Disabled;
use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;

use e3_events::BusHandle;
use e3_events::Event;
use e3_events::EventBus;
use e3_events::InterfoldEvent;

use crate::EventSystem;

/// Process-wide cache of one event bus per event type.
pub struct EventBusFactory {
    event_bus_cache: Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl EventBusFactory {
    // Get the singleton instance of the factory
    pub fn instance() -> &'static EventBusFactory {
        static INSTANCE: LazyLock<EventBusFactory> = LazyLock::new(|| EventBusFactory {
            event_bus_cache: Mutex::new(HashMap::new()),
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
}

pub fn get_interfold_event_bus() -> Addr<EventBus<InterfoldEvent>> {
    EventBusFactory::instance().get_event_bus()
}

pub fn get_interfold_bus_handle() -> anyhow::Result<BusHandle<Disabled>> {
    let bus = get_interfold_event_bus();
    let system = EventSystem::new().with_event_bus(bus);
    system.store()?; // Ensure store is initialized before returning to avoid potentially dropping
                     // events.
    system.handle()
}
