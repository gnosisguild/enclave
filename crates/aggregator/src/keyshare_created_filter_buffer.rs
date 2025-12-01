use actix::prelude::*;

use e3_events::{prelude::*, EnclaveEvent, EnclaveEventData};
use std::collections::HashSet;

use crate::PublicKeyAggregator;

pub struct KeyshareCreatedFilterBuffer {
    dest: Addr<PublicKeyAggregator>,
    committee: Option<HashSet<String>>,
    buffer: Vec<EnclaveEvent>,
}

impl KeyshareCreatedFilterBuffer {
    pub fn new(dest: Addr<PublicKeyAggregator>) -> Self {
        Self {
            dest,
            committee: None,
            buffer: Vec::new(),
        }
    }

    fn process_buffered_events(&mut self) {
        if let Some(ref committee) = self.committee {
            for event in self.buffer.drain(..) {
                if let EnclaveEventData::KeyshareCreated(data) = event.get_data() {
                    if committee.contains(&data.node) {
                        self.dest.do_send(event);
                    }
                }
            }
        }
    }
}

impl Actor for KeyshareCreatedFilterBuffer {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for KeyshareCreatedFilterBuffer {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        match msg.get_data() {
            EnclaveEventData::KeyshareCreated(data) => match &self.committee {
                Some(committee) if committee.contains(&data.node) => {
                    // if the committee is ready then process
                    self.dest.do_send(msg);
                }
                None => {
                    // if not buffer
                    self.buffer.push(msg);
                }
                _ => {}
            },
            EnclaveEventData::CommitteeFinalized(data) => {
                self.dest.do_send(msg.clone()); // forward committee first
                self.committee = Some(data.committee.iter().cloned().collect());
                self.process_buffered_events();
            }
            _ => {
                // forward all other events
                self.dest.do_send(msg);
            }
        }
    }
}
