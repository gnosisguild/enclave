use crate::CommitteeMetaFactory;

use super::CommitteeMeta;
use actix::{Actor, Addr, Context, Handler, Recipient};
use aggregator::PlaintextAggregator;
use aggregator::PublicKeyAggregator;
use enclave_core::E3RequestComplete;
use enclave_core::{E3id, EnclaveEvent, EventBus, Subscribe};
use fhe::Fhe;
use keyshare::Keyshare;
use std::collections::HashSet;
use std::{collections::HashMap, sync::Arc};

/// Helper class to buffer events for downstream instances incase events arrive in the wrong order
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

/// Context that is set to each event hook. Hooks can use this context to gather dependencies if
/// they need to instantiate struct instances or actors.
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

/// Format of the hook that needs to be passed to E3RequestRouter
pub type EventHook = Box<dyn FnMut(&mut E3RequestContext, EnclaveEvent)>;

/// E3RequestRouter will register hooks that receive an E3_id specific context. After hooks
/// have run e3_id specific messages are forwarded to all instances on the context. This enables
/// hooks to lazily register instances that have the correct dependencies available per e3_id
/// request
// TODO: setup typestate pattern so that we have to place hooks within correct order of
// dependencies
pub struct E3RequestRouter {
    contexts: HashMap<E3id, E3RequestContext>,
    completed: HashSet<E3id>,
    hooks: Vec<EventHook>,
    buffer: EventBuffer,
    bus: Addr<EventBus>,
}

impl E3RequestRouter {
    pub fn builder(bus: Addr<EventBus>) -> E3RequestRouterBuilder {
        let builder = E3RequestRouterBuilder { bus, hooks: vec![] };

        // Everything needs the committe meta factory so adding it here by default
        builder.add_hook(CommitteeMetaFactory::create())
    }
}

impl Actor for E3RequestRouter {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for E3RequestRouter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        let Some(e3_id) = msg.get_e3_id() else {
            return;
        };

        if self.completed.contains(&e3_id) {
            // TODO: Log warning that e3 event was received for completed e3_id
            return;
        }

        let context = self.contexts.entry(e3_id.clone()).or_default();

        for hook in &mut self.hooks {
            hook(context, msg.clone());
        }

        context.forward_message(&msg, &mut self.buffer);

        match &msg {
            EnclaveEvent::PlaintextAggregated { .. } => {
                // Here we are detemining that by receiving the PlaintextAggregated event our request is
                // complete and we can notify everyone. This might change as we consider other factors
                // when determining if the request is complete
                let event = EnclaveEvent::from(E3RequestComplete {
                    e3_id: e3_id.clone(),
                });

                // Send to bus so all other actors can react to a request being complete.
                self.bus.do_send(event);
            }
            EnclaveEvent::E3RequestComplete { .. } => {
                // Note this will be sent above to the children who can kill themselves based on
                // the event
                self.contexts.remove(&e3_id);
                self.completed.insert(e3_id);
            }
            _ => (),
        }
    }
}

/// Builder for E3RequestRouter
pub struct E3RequestRouterBuilder {
    pub bus: Addr<EventBus>,
    pub hooks: Vec<EventHook>,
}
impl E3RequestRouterBuilder {
    pub fn add_hook(mut self, listener: EventHook) -> Self {
        self.hooks.push(listener);
        self
    }

    pub fn build(self) -> Addr<E3RequestRouter> {
        let e3r = E3RequestRouter {
            contexts: HashMap::new(),
            completed: HashSet::new(),
            hooks: self.hooks,
            buffer: EventBuffer::default(),
            bus: self.bus.clone(),
        };

        let addr = e3r.start();
        self.bus
            .do_send(Subscribe::new("*", addr.clone().recipient()));
        addr
    }
}
