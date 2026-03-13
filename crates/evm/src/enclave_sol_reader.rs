// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::events::EvmEventProcessor;
use crate::evm_parser::EvmParser;
use actix::{Actor, Addr};
use alloy::primitives::{LogData, B256};
use alloy::{sol, sol_types::SolEvent};
use e3_events::E3id;
use e3_events::EnclaveEventData;
use e3_events::{E3Failed, E3Stage, E3StageChanged, FailureReason};
use e3_fhe_params::decode_bfv_params_arc;
use e3_trbfv::helpers::calculate_error_size;
use e3_utils::ArcBytes;
use e3_zk_helpers::CiphernodesCommitteeSize;
use num_bigint::BigUint;
use tracing::{error, info, trace, warn};

sol!(
    #[sol(rpc)]
    IEnclave,
    "../../packages/enclave-contracts/artifacts/contracts/interfaces/IEnclave.sol/IEnclave.json"
);

struct E3RequestedWithChainId(pub IEnclave::E3Requested, pub u64);

impl E3RequestedWithChainId {
    fn try_into_e3_requested(self) -> anyhow::Result<e3_events::E3Requested> {
        let params_bytes = self.0.e3.e3ProgramParams.to_vec();

        // Derive threshold values from committee size enum
        let committee_size = match self.0.e3.committeeSize {
            0 => CiphernodesCommitteeSize::Micro,
            1 => CiphernodesCommitteeSize::Small,
            2 => CiphernodesCommitteeSize::Medium,
            3 => CiphernodesCommitteeSize::Large,
            other => anyhow::bail!("Unsupported committee size: {}", other),
        };
        let committee = committee_size.values();
        let threshold_m = committee.threshold;
        let threshold_n = committee.n;

        let params_arc = decode_bfv_params_arc(&params_bytes).expect("Failed to decode BFV params");

        // TODO: These should be delivered from the e3_program contract
        // For now, using defaults that match the test configuration:
        // - lambda = 2 (INSECURE, for testing only. Production should use lambda = 80)
        // - esi_per_ct = 3 (number of ciphertexts per encryption slot)
        let lambda = 2;
        let esi_per_ct = 3;

        let error_size = match calculate_error_size(
            params_arc.clone(),
            threshold_n,
            threshold_m,
            lambda,
        ) {
            Ok(size) => {
                let size_bytes = size.to_bytes_be();
                info!(
                    "Calculated error_size for E3 (threshold_n={}, threshold_m={}, lambda={}): {} bytes",
                    threshold_n, threshold_m, lambda, size_bytes.len()
                );
                ArcBytes::from_bytes(&size_bytes)
            }
            Err(e) => {
                warn!(
                    "Failed to calculate error_size, using fallback: {}. \
                    This may cause decryption failures!",
                    e
                );
                ArcBytes::from_bytes(
                    &BigUint::from(36128399948547143872891754381312u128).to_bytes_be(),
                )
            }
        };

        Ok(e3_events::E3Requested {
            params: ArcBytes::from_bytes(&params_bytes),
            threshold_m,
            threshold_n,
            seed: self.0.e3.seed.into(),
            error_size,
            esi_per_ct,
            e3_id: E3id::new(self.0.e3Id.to_string(), self.1),
        })
    }
}

struct CiphertextOutputPublishedWithChainId(pub IEnclave::CiphertextOutputPublished, pub u64);

impl From<CiphertextOutputPublishedWithChainId> for e3_events::CiphertextOutputPublished {
    fn from(value: CiphertextOutputPublishedWithChainId) -> Self {
        e3_events::CiphertextOutputPublished {
            e3_id: E3id::new(value.0.e3Id.to_string(), value.1),
            // XXX: Ciphertext is an array of bytes this needs to be coordinated with enclave
            // contract
            ciphertext_output: vec![ArcBytes::from_bytes(&value.0.ciphertextOutput.to_vec())],
        }
    }
}

impl From<CiphertextOutputPublishedWithChainId> for EnclaveEventData {
    fn from(value: CiphertextOutputPublishedWithChainId) -> Self {
        let payload: e3_events::CiphertextOutputPublished = value.into();
        payload.into()
    }
}

struct E3FailedWithChainId(pub IEnclave::E3Failed, pub u64);

fn convert_u8_to_e3_stage(stage_u8: u8) -> E3Stage {
    match stage_u8 {
        0 => E3Stage::None,
        1 => E3Stage::Requested,
        2 => E3Stage::CommitteeFinalized,
        3 => E3Stage::KeyPublished,
        4 => E3Stage::CiphertextReady,
        5 => E3Stage::Complete,
        6 => E3Stage::Failed,
        _ => E3Stage::None,
    }
}

// Helper function to convert u8 to Rust FailureReason
fn convert_u8_to_failure_reason(reason_u8: u8) -> FailureReason {
    match reason_u8 {
        0 => FailureReason::None,
        1 => FailureReason::CommitteeFormationTimeout,
        2 => FailureReason::InsufficientCommitteeMembers,
        3 => FailureReason::DKGTimeout,
        4 => FailureReason::DKGInvalidShares,
        5 => FailureReason::NoInputsReceived,
        6 => FailureReason::ComputeTimeout,
        7 => FailureReason::ComputeProviderExpired,
        8 => FailureReason::ComputeProviderFailed,
        9 => FailureReason::RequesterCancelled,
        10 => FailureReason::DecryptionTimeout,
        11 => FailureReason::DecryptionInvalidShares,
        12 => FailureReason::VerificationFailed,
        _ => FailureReason::None,
    }
}

impl From<E3FailedWithChainId> for E3Failed {
    fn from(value: E3FailedWithChainId) -> Self {
        E3Failed {
            e3_id: E3id::new(value.0.e3Id.to_string(), value.1),
            failed_at_stage: convert_u8_to_e3_stage(value.0.failedAtStage),
            reason: convert_u8_to_failure_reason(value.0.reason),
        }
    }
}

impl From<E3FailedWithChainId> for EnclaveEventData {
    fn from(value: E3FailedWithChainId) -> Self {
        let payload: E3Failed = value.into();
        payload.into()
    }
}

struct E3StageChangedWithChainId(pub IEnclave::E3StageChanged, pub u64);

impl From<E3StageChangedWithChainId> for E3StageChanged {
    fn from(value: E3StageChangedWithChainId) -> Self {
        E3StageChanged {
            e3_id: E3id::new(value.0.e3Id.to_string(), value.1),
            previous_stage: convert_u8_to_e3_stage(value.0.previousStage),
            new_stage: convert_u8_to_e3_stage(value.0.newStage),
        }
    }
}

impl From<E3StageChangedWithChainId> for EnclaveEventData {
    fn from(value: E3StageChangedWithChainId) -> Self {
        let payload: E3StageChanged = value.into();
        payload.into()
    }
}

pub fn extractor(data: &LogData, topic: Option<&B256>, chain_id: u64) -> Option<EnclaveEventData> {
    match topic {
        Some(&IEnclave::E3Requested::SIGNATURE_HASH) => {
            let Ok(event) = IEnclave::E3Requested::decode_log_data(data) else {
                error!("Error parsing event E3Requested after topic matched!");
                return None;
            };
            match E3RequestedWithChainId(event, chain_id).try_into_e3_requested() {
                Ok(payload) => Some(payload.into()),
                Err(e) => {
                    error!("Error processing E3Requested event: {}", e);
                    None
                }
            }
        }
        Some(&IEnclave::CiphertextOutputPublished::SIGNATURE_HASH) => {
            let Ok(event) = IEnclave::CiphertextOutputPublished::decode_log_data(data) else {
                error!("Error parsing event CiphertextOutputPublished after topic matched!");
                return None;
            };
            Some(EnclaveEventData::from(
                CiphertextOutputPublishedWithChainId(event, chain_id),
            ))
        }
        Some(&IEnclave::E3Failed::SIGNATURE_HASH) => {
            let Ok(event) = IEnclave::E3Failed::decode_log_data(data) else {
                error!("Error parsing event E3Failed after topic matched!");
                return None;
            };
            info!(
                "E3Failed event received: e3_id={}, stage={:?}, reason={:?}",
                event.e3Id, event.failedAtStage, event.reason
            );
            Some(EnclaveEventData::from(E3FailedWithChainId(event, chain_id)))
        }
        Some(&IEnclave::E3StageChanged::SIGNATURE_HASH) => {
            let Ok(event) = IEnclave::E3StageChanged::decode_log_data(data) else {
                error!("Error parsing event E3StageChanged after topic matched!");
                return None;
            };
            trace!(
                "E3StageChanged event received: e3_id={}, {:?} -> {:?}",
                event.e3Id,
                event.previousStage,
                event.newStage
            );
            Some(EnclaveEventData::from(E3StageChangedWithChainId(
                event, chain_id,
            )))
        }
        _topic => {
            trace!(
                topic=?_topic,
                "Unknown event received by Enclave.sol parser but was ignored"
            );
            None
        }
    }
}

/// Connects to Enclave.sol converting EVM events to EnclaveEvents
pub struct EnclaveSolReader;

impl EnclaveSolReader {
    pub fn setup(next: &EvmEventProcessor) -> Addr<EvmParser> {
        EvmParser::new(next, extractor).start()
    }
}
