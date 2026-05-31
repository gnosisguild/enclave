// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;

use e3_events::{prelude::*, AggregatorChanged, Die, EnclaveEvent, EnclaveEventData};
use e3_utils::MAILBOX_LIMIT;
use std::collections::HashSet;
use tracing::info;

use crate::PublicKeyAggregator;

/// Buffers `KeyshareCreated` events until `CommitteeFinalized` arrives.
pub struct KeyshareCreatedFilterBuffer {
    dest: Addr<PublicKeyAggregator>,
    committee: Option<HashSet<String>>,
    buffer: Vec<EnclaveEvent>,
    expelled_nodes: HashSet<String>,
    is_aggregator: bool,
}

impl KeyshareCreatedFilterBuffer {
    pub fn new(dest: Addr<PublicKeyAggregator>) -> Self {
        Self {
            dest,
            committee: None,
            buffer: Vec::new(),
            expelled_nodes: HashSet::new(),
            is_aggregator: false,
        }
    }

    fn process_buffered_events(&mut self) {
        if !self.is_aggregator {
            return;
        }

        if let Some(ref committee) = self.committee {
            for event in self.buffer.drain(..) {
                match event.get_data() {
                    EnclaveEventData::KeyshareCreated(data)
                        if committee.contains(&data.node)
                            && !self.expelled_nodes.contains(&data.node) =>
                    {
                        self.dest.do_send(event);
                    }
                    EnclaveEventData::CommitteeMemberExpelled(data) if data.party_id.is_none() => {
                        self.dest.do_send(event);
                    }
                    EnclaveEventData::E3RequestComplete(_) | EnclaveEventData::Shutdown(_) => {
                        self.dest.do_send(event);
                    }
                    _ => {}
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
                Some(committee)
                    if self.is_aggregator
                        && committee.contains(&data.node)
                        && !self.expelled_nodes.contains(&data.node) =>
                {
                    self.dest.do_send(msg);
                }
                None => {
                    self.buffer.push(msg);
                }
                Some(committee)
                    if committee.contains(&data.node)
                        && !self.expelled_nodes.contains(&data.node) =>
                {
                    self.buffer.push(msg);
                }
                _ => {}
            },
            EnclaveEventData::CommitteeFinalized(data) => {
                self.committee = Some(data.committee.iter().cloned().collect());
                self.process_buffered_events();
            }
            EnclaveEventData::CommitteeMemberExpelled(data) => {
                if data.party_id.is_some() {
                    return;
                }

                let node_addr = data.node.to_string();
                self.expelled_nodes.insert(node_addr.clone());
                self.buffer.retain(|event| {
                    !matches!(
                        event.get_data(),
                        EnclaveEventData::KeyshareCreated(share) if share.node == node_addr
                    )
                });

                if let Some(ref mut committee) = self.committee {
                    info!(
                        "KeyshareCreatedFilterBuffer: removing expelled node {} from committee filter (e3_id={})",
                        node_addr, data.e3_id
                    );
                    committee.remove(&node_addr);
                }

                if self.is_aggregator {
                    self.dest.do_send(msg);
                } else {
                    self.buffer.push(msg);
                }
            }
            EnclaveEventData::AggregatorChanged(AggregatorChanged { is_aggregator, .. }) => {
                self.is_aggregator = *is_aggregator;
                self.process_buffered_events();
            }
            EnclaveEventData::E3RequestComplete(_) | EnclaveEventData::Shutdown(_) => {
                self.dest.do_send(msg);
            }
            _ => {
                if self.is_aggregator {
                    self.dest.do_send(msg);
                }
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
