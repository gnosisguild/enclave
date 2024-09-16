// TODO: spawn and supervise child actors
use crate::{
    CommitteeMeta, E3id, EnclaveEvent, EventBus, Fhe, InitializeWithEnclaveEvent,
    PlaintextAggregator, Sortition,
};
use actix::prelude::*;
use std::collections::HashMap;

pub struct PlaintextRegistry {
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
    buffers: HashMap<E3id, Vec<EnclaveEvent>>,
    plaintexts: HashMap<E3id, Addr<PlaintextAggregator>>,
}

impl PlaintextRegistry {
    pub fn new(bus: Addr<EventBus>, sortition: Addr<Sortition>) -> Self {
        Self {
            bus,
            sortition,
            plaintexts: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn attach(bus: Addr<EventBus>, sortition: Addr<Sortition>) -> Addr<Self> {
        PlaintextRegistry::new(bus.clone(), sortition).start()
    }
}

impl Actor for PlaintextRegistry {
    type Context = Context<Self>;
}

impl Handler<InitializeWithEnclaveEvent> for PlaintextRegistry {
    type Result = ();
    fn handle(&mut self, msg: InitializeWithEnclaveEvent, _: &mut Self::Context) -> Self::Result {
        let InitializeWithEnclaveEvent { fhe, meta, event } = msg;
        let EnclaveEvent::CiphertextOutputPublished { data, .. } = event else {
            return;
        };

        let plaintext_factory =
            self.plaintext_factory(data.e3_id.clone(), meta.clone(), fhe.clone());

        self.plaintexts
            .entry(data.e3_id)
            .or_insert_with(plaintext_factory);
    }
}

impl Handler<EnclaveEvent> for PlaintextRegistry {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        let Some(e3_id) = msg.get_e3_id() else {
            return;
        };
        self.forward_message(&e3_id, msg);
    }
}

impl PlaintextRegistry {
    fn plaintext_factory(
        &self,
        e3_id: E3id,
        meta: CommitteeMeta,
        fhe: Addr<Fhe>,
    ) -> impl FnOnce() -> Addr<PlaintextAggregator> {
        let bus = self.bus.clone();
        let sortition = self.sortition.clone();
        let nodecount = meta.nodecount;
        let seed = meta.seed;
        move || PlaintextAggregator::new(fhe, bus, sortition, e3_id, nodecount, seed).start()
    }

    fn store_msg(&mut self, e3_id: E3id, msg: EnclaveEvent) {
        self.buffers.entry(e3_id).or_default().push(msg);
    }

    fn take_msgs(&mut self, e3_id: E3id) -> Vec<EnclaveEvent> {
        self.buffers
            .get_mut(&e3_id)
            .map(std::mem::take)
            .unwrap_or_default()
    }

    fn forward_message(&mut self, e3_id: &E3id, msg: EnclaveEvent) {
        // Buffer events for each thing that has not been created
        if let Some(act) = self.plaintexts.clone().get(e3_id) {
            let msgs = self.take_msgs(e3_id.clone());
            let recipient = act.clone().recipient();
            recipient.do_send(msg.clone());
            for m in msgs {
                recipient.do_send(m);
            }
        } else {
            self.store_msg(e3_id.clone(), msg.clone());
        }
    }
}
