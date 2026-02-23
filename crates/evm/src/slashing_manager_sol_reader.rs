// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    events::EvmEventProcessor, evm_parser::EvmParser, slashing_manager_sol_writer::ISlashingManager,
};
use actix::{Actor, Addr};
use alloy::{
    primitives::{LogData, B256},
    sol_types::SolEvent,
};
use e3_events::{E3id, EnclaveEventData};
use tracing::{error, info, trace};

pub fn extractor(data: &LogData, topic: Option<&B256>, chain_id: u64) -> Option<EnclaveEventData> {
    match topic {
        Some(&ISlashingManager::SlashExecuted::SIGNATURE_HASH) => {
            let Ok(event) = ISlashingManager::SlashExecuted::decode_log_data(data) else {
                error!("Error parsing event SlashExecuted after topic was matched!");
                return None;
            };
            info!(
                "SlashExecuted event received: proposal_id={}, e3_id={}, operator={}, reason={:?}, ticket={}, license={}",
                event.proposalId, event.e3Id, event.operator, event.reason, event.ticketAmount, event.licenseAmount
            );
            Some(EnclaveEventData::from(e3_events::SlashExecuted {
                e3_id: E3id::new(event.e3Id.to_string(), chain_id),
                proposal_id: event.proposalId.to::<u128>(),
                operator: event.operator,
                reason: event.reason.into(),
                ticket_amount: event.ticketAmount.to::<u128>(),
                license_amount: event.licenseAmount.to::<u128>(),
            }))
        }
        _topic => {
            trace!(
                topic=?_topic,
                "Unknown event was received by SlashingManager.sol parser but was ignored"
            );
            None
        }
    }
}

/// Connects to SlashingManager.sol converting EVM events to EnclaveEvents
pub struct SlashingManagerSolReader;

impl SlashingManagerSolReader {
    pub fn setup(next: &EvmEventProcessor) -> Addr<EvmParser> {
        EvmParser::new(next, extractor).start()
    }
}
