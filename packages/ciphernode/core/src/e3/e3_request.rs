use std::{collections::HashMap, sync::Arc};

use actix::{Actor, Addr, Context, Handler, Recipient};

use crate::{enclave_core::{E3id, EnclaveEvent, EventBus, Subscribe}, fhe::Fhe, keyshare::Keyshare, plaintext_aggregator::PlaintextAggregator, publickey_aggregator::PublicKeyAggregator};

use super::CommitteeMeta;

#[derive(Default)]
struct EventBuffer {
    buffer: HashMap<String, Vec<EnclaveEvent>>,
}

impl EventBuffer {
    pub fn add(&mut self, key: &str, event: EnclaveEvent) {
        self.buffer.entry(key.to_string()).or_default().push(event)
    }

    pub fn take(&mut self, key: &str) -> Vec<EnclaveEvent> {
        self.buffer
            .get_mut(key)
            .map(std::mem::take)
            .unwrap_or_default()
    }
}

#[derive(Default)]
pub struct E3RequestContext {
    pub keyshare: Option<Addr<Keyshare>>,
    pub fhe: Option<Arc<Fhe>>,
    pub plaintext: Option<Addr<PlaintextAggregator>>,
    pub publickey: Option<Addr<PublicKeyAggregator>>,
    pub meta: Option<CommitteeMeta>,
}


impl E3RequestContext {
    fn recipients(&self) -> Vec<(String, Option<Recipient<EnclaveEvent>>)> {
        vec![
            (
                "keyshare".to_owned(),
                self.keyshare.clone().map(|addr| addr.into()),
            ),
            (
                "plaintext".to_owned(),
                self.plaintext.clone().map(|addr| addr.into()),
            ),
            (
                "publickey".to_owned(),
                self.publickey.clone().map(|addr| addr.into()),
            ),
        ]
    }

    fn forward_message(&self, msg: &EnclaveEvent, buffer: &mut EventBuffer) {
        self.recipients().into_iter().for_each(|(key, recipient)| {
            if let Some(act) = recipient {
                act.do_send(msg.clone());
                for m in buffer.take(&key) {
                    act.do_send(m);
                }
            } else {
                buffer.add(&key, msg.clone());
            }
        });
    }
}

pub type EventHook = Box<dyn FnMut(&mut E3RequestContext, EnclaveEvent)>;

// TODO: setup typestate pattern so that we have to place hooks within correct order of
// dependencies
pub struct E3RequestManager {
    contexts: HashMap<E3id, E3RequestContext>,
    hooks: Vec<EventHook>,
    buffer: EventBuffer,
}

impl E3RequestManager {
    pub fn builder(bus: Addr<EventBus>) -> E3RequestManagerBuilder {
        E3RequestManagerBuilder {
            bus,
            hooks: vec![],
        }
    }
}

pub struct E3RequestManagerBuilder {
    pub bus: Addr<EventBus>,
    pub hooks: Vec<EventHook>,
}
impl E3RequestManagerBuilder {
    pub fn add_hook(mut self, listener: EventHook) -> Self {
        self.hooks.push(listener);
        self
    }

    pub fn build(self) -> Addr<E3RequestManager> {
        let e3r = E3RequestManager {
            contexts: HashMap::new(),
            hooks: self.hooks,
            buffer: EventBuffer::default(),
        };

        let addr = e3r.start();
        self.bus
            .do_send(Subscribe::new("*", addr.clone().recipient()));
        addr
    }
}

impl Actor for E3RequestManager {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for E3RequestManager {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        let Some(e3_id) = msg.get_e3_id() else {
            return;
        };

        let context = self.contexts.entry(e3_id).or_default();

        for hook in &mut self.hooks {
            hook(context, msg.clone());
        }

        context.forward_message(&msg, &mut self.buffer);
    }
}
