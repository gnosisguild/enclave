// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;

use e3_events::{prelude::*, Die, EnclaveEvent, EnclaveEventData};
use e3_utils::MAILBOX_LIMIT;
use std::collections::HashSet;
use tracing::info;

use crate::PublicKeyAggregator;

/// Buffers `KeyshareCreated` events until `CommitteeFinalized` arrives.
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
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
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
                self.dest.do_send(msg.clone());
                self.committee = Some(data.committee.iter().cloned().collect());
                self.process_buffered_events();
            }
            EnclaveEventData::CommitteeMemberExpelled(data) => {
                // Only process raw events from chain (party_id not yet resolved).
                if data.party_id.is_some() {
                    return;
                }

                // Remove expelled node so we don't forward late KeyshareCreated events from them
                if let Some(ref mut committee) = self.committee {
                    let node_addr = data.node.to_string();
                    info!(
                        "KeyshareCreatedFilterBuffer: removing expelled node {} from committee filter (e3_id={})",
                        node_addr, data.e3_id
                    );
                    committee.remove(&node_addr);
                }
                // Forward to PublicKeyAggregator for threshold_n adjustment
                self.dest.do_send(msg);
            }
            _ => {
                self.dest.do_send(msg);
            }
        }
    }
}

impl Handler<Die> for KeyshareCreatedFilterBuffer {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}
