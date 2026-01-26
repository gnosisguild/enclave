use actix::{Actor, Handler};
use e3_events::{hlc::HlcTimestamp, EnclaveEventData, EvmEvent};
use tracing::info;

use crate::{
    events::{EnclaveEvmEvent, EvmEventProcessor, EvmLog},
    ExtractorFn,
};

pub struct EvmReader {
    next: EvmEventProcessor,
    extractor: ExtractorFn<EnclaveEventData>,
}

impl Actor for EvmReader {
    type Context = actix::Context<Self>;
}

impl EvmReader {
    pub fn new(next: &EvmEventProcessor, extractor: ExtractorFn<EnclaveEventData>) -> Self {
        Self {
            next: next.clone(),
            extractor,
        }
    }
}

impl Handler<EnclaveEvmEvent> for EvmReader {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvmEvent, _ctx: &mut Self::Context) -> Self::Result {
        match msg.clone() {
            EnclaveEvmEvent::Log(EvmLog { log, chain_id, id }) => {
                let extractor = self.extractor;

                if let Some(event) = extractor(log.data(), log.topic0(), chain_id) {
                    let err = "Log should always have metadata because we listen to non-pending blocks. If you are seeing this it is likely because there is an issue with how we are subscribing to blocks";
                    let block = log.block_number.expect(err);
                    let block_timestamp = log.block_timestamp.expect(err);
                    let log_index = log.log_index.expect(err);
                    let ts = from_log_chain_id_to_ts(block_timestamp, log_index, chain_id);
                    self.next.do_send(EnclaveEvmEvent::Event(EvmEvent::new(
                        // note we use the id from the log event above!
                        id, event, block, ts, chain_id,
                    )))
                }
            }
            hist @ EnclaveEvmEvent::HistoricalSyncComplete(..) => self.next.do_send(hist),
            _ => (),
        }
    }
}

fn from_log_chain_id_to_ts(block_timestamp: u64, log_index: u64, chain_id: u64) -> u128 {
    let ts = block_timestamp.saturating_mul(1_000_000);

    // Use log_index as counter (orders logs within same block)
    let counter = log_index as u32;

    // Use transaction_index as node (or chain_id if you have it)
    let node = chain_id as u32;

    HlcTimestamp::new(ts, counter, node).into()
}
