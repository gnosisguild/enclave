// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Handler};
use alloy::primitives::{LogData, B256};
use e3_events::EnclaveEventData;
use e3_utils::MAILBOX_LIMIT;
use tracing::debug;

use crate::domain::log_timestamp::from_log_chain_id_to_ts;
use crate::messages::{EnclaveEvmEvent, EvmEvent, EvmEventProcessor, EvmLog};

pub type ExtractorFn<E> = fn(&LogData, &[B256], u64) -> Option<E>;

pub struct EvmParser {
    next: EvmEventProcessor,
    extractor: ExtractorFn<EnclaveEventData>,
}

impl Actor for EvmParser {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT)
    }
}

impl EvmParser {
    pub fn new(next: &EvmEventProcessor, extractor: ExtractorFn<EnclaveEventData>) -> Self {
        Self {
            next: next.clone(),
            extractor,
        }
    }
}

impl Handler<EnclaveEvmEvent> for EvmParser {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvmEvent, _ctx: &mut Self::Context) -> Self::Result {
        match msg.clone() {
            EnclaveEvmEvent::Log(EvmLog {
                log,
                chain_id,
                id,
                timestamp,
            }) => {
                debug!("processing event({})", msg.get_id());
                let extractor = self.extractor;

                if let Some(event) = extractor(log.data(), log.topics(), chain_id) {
                    let err = "Log should always have metadata because we listen to non-pending blocks. If you are seeing this it is likely because there is an issue with how we are subscribing to blocks";
                    let block = log.block_number.expect(err);
                    let log_index = log.log_index.expect(err);
                    let ts = from_log_chain_id_to_ts(timestamp, log_index, chain_id);
                    self.next.do_send(EnclaveEvmEvent::Event(EvmEvent::new(
                        // note we use the id from the log event above!
                        id, event, block, ts, chain_id,
                    )))
                } else {
                    self.next.do_send(EnclaveEvmEvent::Processed(id))
                }
            }
            hist @ EnclaveEvmEvent::HistoricalSyncComplete(..) => self.next.do_send(hist),
            _ => (),
        }
    }
}
