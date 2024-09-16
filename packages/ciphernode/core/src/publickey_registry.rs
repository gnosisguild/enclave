// TODO: spawn and supervise child actors
use crate::{
    E3id, EnclaveEvent, EventBus, Fhe, InitializeWithEnclaveEvent, PublicKeyAggregator, Sortition,
};
use actix::prelude::*;
use std::collections::HashMap;

pub struct PublicKeyRegistry {
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
    buffers: HashMap<E3id, Vec<EnclaveEvent>>,
    public_keys: HashMap<E3id, Addr<PublicKeyAggregator>>,
}

impl PublicKeyRegistry {
    pub fn new(bus: Addr<EventBus>, sortition: Addr<Sortition>) -> Self {
        Self {
            bus,
            sortition,
            public_keys: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn attach(bus: Addr<EventBus>, sortition: Addr<Sortition>) -> Addr<Self> {
        PublicKeyRegistry::new(bus.clone(), sortition).start()
    }
}

impl Actor for PublicKeyRegistry {
    type Context = Context<Self>;
}

impl Handler<InitializeWithEnclaveEvent> for PublicKeyRegistry {
    type Result = ();
    fn handle(&mut self, msg: InitializeWithEnclaveEvent, _: &mut Self::Context) -> Self::Result {
        let InitializeWithEnclaveEvent { fhe, event, .. } = msg;
        let EnclaveEvent::CommitteeRequested { data, .. } = event else {
            return;
        };

        let public_key_factory = self.public_key_factory(
            fhe.clone(),
            data.e3_id.clone(),
            data.nodecount,
            data.sortition_seed,
        );
        self.public_keys
            .entry(data.e3_id)
            .or_insert_with(public_key_factory);
    }
}

impl Handler<EnclaveEvent> for PublicKeyRegistry {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        let Some(e3_id) = msg.get_e3_id() else {
            return;
        };
        self.forward_message(&e3_id, msg);
    }
}

impl PublicKeyRegistry {
    fn public_key_factory(
        &self,
        fhe: Addr<Fhe>,
        e3_id: E3id,
        nodecount: usize,
        seed: u64,
    ) -> impl FnOnce() -> Addr<PublicKeyAggregator> {
        let bus = self.bus.clone();
        let sortition = self.sortition.clone();
        move || PublicKeyAggregator::new(fhe, bus, sortition, e3_id, nodecount, seed).start()
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
        if let Some(act) = self.public_keys.clone().get(e3_id) {
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
