// TODO: spawn and supervise child actors
use crate::{Ciphernode, Data, E3id, EnclaveEvent, EventBus, Fhe, InitializeWithEnclaveEvent};
use actix::prelude::*;
use alloy_primitives::Address;
use std::collections::HashMap;

pub struct CiphernodeOrchestrator {
    bus: Addr<EventBus>,
    data: Addr<Data>,
    address: Address,
    ciphernodes: HashMap<E3id, Addr<Ciphernode>>,
    buffers: HashMap<E3id, Vec<EnclaveEvent>>,
}

impl CiphernodeOrchestrator {
    pub fn new(bus: Addr<EventBus>, data: Addr<Data>, address: Address) -> Self {
        Self {
            bus,
            data,
            address,
            ciphernodes: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn attach(bus: Addr<EventBus>, data: Addr<Data>, address: Address) -> Addr<Self> {
        CiphernodeOrchestrator::new(bus, data, address).start()
    }
}

impl Actor for CiphernodeOrchestrator {
    type Context = Context<Self>;
}

impl Handler<InitializeWithEnclaveEvent> for CiphernodeOrchestrator {
    type Result = ();
    fn handle(&mut self, msg: InitializeWithEnclaveEvent, _: &mut Self::Context) -> Self::Result {
        let InitializeWithEnclaveEvent { fhe, event, .. } = msg;
        let EnclaveEvent::CommitteeRequested { data, .. } = event else {
            return;
        };
        let ciphernode_factory = self.ciphernode_factory(fhe.clone());
        self.ciphernodes
            .entry(data.e3_id.clone())
            .or_insert_with(ciphernode_factory);
    }
}

impl Handler<EnclaveEvent> for CiphernodeOrchestrator {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        let Some(e3_id) = msg.get_e3_id() else {
            return;
        };
        self.forward_message(&e3_id, msg);
    }
}

impl CiphernodeOrchestrator {
    fn ciphernode_factory(&self, fhe: Addr<Fhe>) -> impl FnOnce() -> Addr<Ciphernode> {
        let data = self.data.clone();
        let bus = self.bus.clone();
        let address = self.address;
        move || Ciphernode::new(bus, fhe, data, address).start()
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
        if let Some(act) = self.ciphernodes.clone().get(e3_id) {
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
