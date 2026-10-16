// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Subscribes to `AccusationQuorumReached` events and submits committee-attested
//! slash proposals on the SlashingManager contract. Prefers party-attributed
//! `proposeSlashByDkgParty` when DKG anchors resolve, and falls back to
//! operator-attributed `proposeSlash` otherwise.

use crate::contracts::{ICiphernodeRegistry, ISlashingManager};
use crate::domain::attestation_evidence::encode_attestation_evidence;
use crate::domain::error_decoder::format_evm_error;
use crate::domain::slash_submission::{
    classify_slash_policy, is_slashable_outcome, slash_reason, SlashIntentKey, SlashSubmission,
    SlashSubmissionDecision, SlashSubmissionGate, SubmissionReport, SubmissionStep,
};
use crate::helpers::{transaction_nonce_guard, EthProvider};
use crate::send_tx_with_retry;
use actix::prelude::*;
use actix::Addr;
use alloy::{
    primitives::{Address, Bytes, U256},
    providers::{Provider, WalletProvider},
    rpc::types::TransactionReceipt,
};
use anyhow::Result;
use e3_data::{AutoPersist, Persistable, Repository};
use e3_events::prelude::*;
use e3_events::BusHandle;
use e3_events::EventType;
use e3_events::InterfoldEvent;
use e3_events::InterfoldEventData;
use e3_events::Shutdown;
use e3_events::{
    AccusationQuorumReached, CommitteeMemberExcluded, EType, EventContext, Sequenced, SlashExecuted,
};
use e3_utils::{require_successful_receipt, NotifySync, MAILBOX_LIMIT};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tracing::{info, warn};

#[path = "effects.rs"]
mod effects;
#[path = "handlers.rs"]
mod handlers;

/// Submits `AccusationQuorumReached` events as slash proposals on-chain.
pub struct SlashingManagerSolWriter<P> {
    provider: EthProvider<P>,
    contract_address: Address,
    bus: BusHandle,
    submissions: SlashSubmissionGate,
    recovery: Option<Persistable<SlashingWriterRecoveryState>>,
    intent_times: Option<Persistable<SlashIntentTimes>>,
}

const SLASH_SUBMISSION_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(30);
pub const SLASHING_WRITER_RECOVERY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlashingWriterRecoveryState {
    pub schema_version: u32,
    pending: BTreeMap<SlashIntentKey, AccusationQuorumReached>,
}

impl Default for SlashingWriterRecoveryState {
    fn default() -> Self {
        Self {
            schema_version: SLASHING_WRITER_RECOVERY_SCHEMA_VERSION,
            pending: BTreeMap::new(),
        }
    }
}

impl SlashingWriterRecoveryState {
    pub fn record(&mut self, event: AccusationQuorumReached) -> Result<()> {
        self.pending
            .entry(SlashIntentKey::from_quorum(&event)?)
            .or_insert(event);
        Ok(())
    }

    fn acknowledge(&mut self, key: &SlashIntentKey) {
        self.pending.remove(key);
    }

    pub fn acknowledge_exclusion(&mut self, event: &CommitteeMemberExcluded) -> Result<()> {
        self.acknowledge(&SlashIntentKey::from_exclusion(event)?);
        Ok(())
    }

    pub fn acknowledge_execution(&mut self, event: &SlashExecuted) -> Result<()> {
        if let Some(key) = SlashIntentKey::from_execution(event)? {
            self.acknowledge(&key);
        }
        Ok(())
    }

    fn pending_events(&self) -> Vec<AccusationQuorumReached> {
        self.pending.values().cloned().collect()
    }

    /// How many slash reports this writer has not yet submitted or seen settled.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

/// When this node first recorded each pending slash intent, in Unix seconds. A fallback submitter
/// counts its delay from this time, so a restart does not start the wait again.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SlashIntentTimes {
    recorded_at: BTreeMap<SlashIntentKey, u64>,
}

impl SlashIntentTimes {
    fn record(&mut self, key: SlashIntentKey, now: u64) {
        self.recorded_at.entry(key).or_insert(now);
    }

    fn acknowledge(&mut self, key: &SlashIntentKey) {
        self.recorded_at.remove(key);
    }

    /// How long ago the intent was first recorded; zero when it was not.
    fn recorded_ago(&self, key: &SlashIntentKey, now: u64) -> std::time::Duration {
        std::time::Duration::from_secs(
            self.recorded_at
                .get(key)
                .map_or(0, |recorded_at| now.saturating_sub(*recorded_at)),
        )
    }
}

pub(super) fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

#[derive(Message)]
#[rtype(result = "()")]
struct SubmitSlashIntent {
    key: SlashIntentKey,
    event: AccusationQuorumReached,
    /// A retry after a failed attempt waits the full rank delay again, so the ranks stay apart.
    retry: bool,
}

#[derive(Message)]
#[rtype(result = "()")]
struct SlashSubmissionFinished {
    key: SlashIntentKey,
    terminal: bool,
    acknowledge_recovery: bool,
    retry_event: Option<AccusationQuorumReached>,
}

impl<P: Provider + WalletProvider + Clone + 'static> SlashingManagerSolWriter<P> {
    pub fn new(
        bus: &BusHandle,
        provider: EthProvider<P>,
        contract_address: Address,
    ) -> Result<Self> {
        Ok(Self {
            provider,
            contract_address,
            bus: bus.clone(),
            submissions: SlashSubmissionGate::new(),
            recovery: None,
            intent_times: None,
        })
    }

    fn from_recovery(
        bus: &BusHandle,
        provider: EthProvider<P>,
        contract_address: Address,
        recovery: Persistable<SlashingWriterRecoveryState>,
        intent_times: Persistable<SlashIntentTimes>,
    ) -> Result<Self> {
        let mut submissions = SlashSubmissionGate::new();
        for event in recovery.try_get()?.pending_events() {
            submissions.admit(event)?;
        }
        Ok(Self {
            provider,
            contract_address,
            bus: bus.clone(),
            submissions,
            recovery: Some(recovery),
            intent_times: Some(intent_times),
        })
    }

    pub async fn attach(
        bus: &BusHandle,
        provider: EthProvider<P>,
        contract_address: Address,
    ) -> Result<Addr<SlashingManagerSolWriter<P>>> {
        let addr = SlashingManagerSolWriter::new(bus, provider, contract_address)?.start();
        bus.subscribe_all(
            &[
                EventType::AccusationQuorumReached,
                EventType::CommitteeMemberExcluded,
                EventType::SlashExecuted,
                EventType::EffectsEnabled,
                EventType::Shutdown,
            ],
            addr.clone().into(),
        );
        Ok(addr)
    }

    pub async fn attach_with_recovery(
        bus: &BusHandle,
        provider: EthProvider<P>,
        contract_address: Address,
        repository: Repository<SlashingWriterRecoveryState>,
        intent_times: Repository<SlashIntentTimes>,
    ) -> Result<Addr<SlashingManagerSolWriter<P>>> {
        let recovery = repository
            .load_or_default(SlashingWriterRecoveryState::default())
            .await?;
        anyhow::ensure!(
            recovery.try_get()?.schema_version == SLASHING_WRITER_RECOVERY_SCHEMA_VERSION,
            "unsupported slashing-writer recovery schema"
        );
        let intent_times = intent_times
            .load_or_default(SlashIntentTimes::default())
            .await?;
        let addr =
            Self::from_recovery(bus, provider, contract_address, recovery, intent_times)?.start();
        bus.subscribe_all(
            &[
                EventType::AccusationQuorumReached,
                EventType::CommitteeMemberExcluded,
                EventType::SlashExecuted,
                EventType::EffectsEnabled,
                EventType::Shutdown,
            ],
            addr.clone().into(),
        );
        Ok(addr)
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> SlashingManagerSolWriter<P> {
    /// Drop the record time of a settled intent.
    fn forget_intent_time(&mut self, key: &SlashIntentKey, ec: Option<&EventContext<Sequenced>>) {
        let Some(intent_times) = self.intent_times.as_mut() else {
            return;
        };
        let forget = |mut times: SlashIntentTimes| {
            times.acknowledge(key);
            Ok(times)
        };
        let result = match ec {
            Some(ec) => intent_times.try_mutate(ec, forget),
            None => intent_times.try_mutate_without_context(forget),
        };
        if let Err(error) = result {
            self.bus.err(EType::Evm, error);
        }
    }
}

impl<P: Provider + WalletProvider + Clone + 'static> Actor for SlashingManagerSolWriter<P> {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_events::{AccusationOutcome, E3id, ProofType};

    fn slash_intent() -> AccusationQuorumReached {
        AccusationQuorumReached {
            e3_id: E3id::new("7", 1),
            accuser: Address::repeat_byte(1),
            accused: Address::repeat_byte(2),
            proof_type: ProofType::C1PkGeneration,
            votes_for: Vec::new(),
            outcome: AccusationOutcome::AccusedFaulted,
            evidence: Bytes::new(),
        }
    }

    #[test]
    fn outbox_coalesces_and_acknowledges() -> Result<()> {
        let intent = slash_intent();
        let mut recovery = SlashingWriterRecoveryState::default();
        recovery.record(intent.clone())?;
        recovery.record(intent.clone())?;
        assert_eq!(recovery.pending_events(), vec![intent.clone()]);

        recovery.acknowledge_exclusion(&CommitteeMemberExcluded {
            e3_id: intent.e3_id.clone(),
            node: intent.accused,
            proof_type: intent.proof_type,
            party_id: None,
        })?;
        assert!(recovery.pending_events().is_empty());

        recovery.record(intent.clone())?;
        recovery.acknowledge_execution(&SlashExecuted {
            e3_id: intent.e3_id,
            proposal_id: 1,
            operator: intent.accused,
            reason: intent.proof_type.attestation_slash_reason().0,
            ticket_amount: 0,
            ciphernode_bond_amount: 0,
        })?;
        assert!(recovery.pending_events().is_empty());
        Ok(())
    }

    /// The first record time of an intent stays, so a later quorum does not restart the wait, and
    /// an acknowledged intent forgets it.
    #[test]
    fn an_intent_keeps_its_first_record_time_until_acknowledged() -> Result<()> {
        let key = SlashIntentKey::from_quorum(&slash_intent())?;
        let mut times = SlashIntentTimes::default();
        assert_eq!(times.recorded_ago(&key, 1_000), std::time::Duration::ZERO);

        times.record(key.clone(), 1_000);
        times.record(key.clone(), 1_030);
        assert_eq!(
            times.recorded_ago(&key, 1_040),
            std::time::Duration::from_secs(40)
        );
        // A clock that went back counts no time.
        assert_eq!(times.recorded_ago(&key, 900), std::time::Duration::ZERO);

        times.acknowledge(&key);
        assert_eq!(times.recorded_ago(&key, 1_040), std::time::Duration::ZERO);
        Ok(())
    }

    /// After the chain confirms a slash, an equivalent quorum that arrives again is not recorded:
    /// the durable state keeps no report that the node has already submitted.
    #[test]
    fn a_completed_intent_is_not_recorded_again() -> Result<()> {
        let intent = slash_intent();
        let mut gate = SlashSubmissionGate::new();
        gate.enable_effects();
        let mut recovery = SlashingWriterRecoveryState::default();
        let (key, decision) =
            gate.record_and_admit(intent.clone(), |event| recovery.record(event.clone()))?;
        assert_eq!(decision, SlashSubmissionDecision::Submit);
        assert_eq!(recovery.pending_count(), 1);

        // The chain confirms the exclusion: the writer acknowledges and completes the intent.
        recovery.acknowledge_exclusion(&CommitteeMemberExcluded {
            e3_id: intent.e3_id.clone(),
            node: intent.accused,
            proof_type: intent.proof_type,
            party_id: None,
        })?;
        gate.mark_completed(key);

        let (_, decision) =
            gate.record_and_admit(intent.clone(), |event| recovery.record(event.clone()))?;
        assert_eq!(decision, SlashSubmissionDecision::IgnoreDuplicate);
        assert_eq!(recovery.pending_count(), 0);
        Ok(())
    }
}
