// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::primitives::{Address, FixedBytes, I256, U256};
use anyhow::Result;
use chrono::Utc;
use clap::Subcommand;
use e3_ciphernode_builder::global_eventstore_cache::EventStoreReader;
use e3_config::AppConfig;
use e3_console::{log, Console};
use e3_crypto::SensitiveBytes;
use e3_entrypoint::helpers::datastore::get_eventstore_reader;
use e3_events::{
    AccusationOutcome, AccusationQuorumReached, AccusationVote, AggregationProofPending,
    AggregationProofSigned, CiphernodeAdded, CiphernodeRemoved, CiphernodeSelected,
    CiphertextOutputPublished, CircuitName, CommitteeFinalizeRequested, CommitteeFinalized,
    CommitteeMemberExpelled, CommitteePublished, CommitteeRequested, ComputeResponse,
    ComputeResponseKind, ConfigurationUpdated, CorrelationId, DKGInnerProofReady,
    DKGRecursiveAggregationComplete, DecryptedSharesAggregationProofRequest, DecryptionKeyShared,
    DecryptionShareProofSigned, DecryptionShareProofsPending, DecryptionshareCreated,
    DkgProofSigned, DkgShareDecryptionProofRequest, DocumentKind, DocumentMeta, DocumentReceived,
    E3Failed, E3RequestComplete, E3Requested, E3Stage, E3StageChanged, E3id, EType, EffectsEnabled,
    EnclaveError, EnclaveEvent, EnclaveEventData, EncryptionKey, EncryptionKeyCollectionFailed,
    EncryptionKeyCreated, EncryptionKeyPending, EncryptionKeyReceived,
    EventConstructorWithTimestamp, EventStoreQueryResponse, EvmEventConfig, EvmEventConfigChain,
    FailureReason, HistoricalEvmSyncStart, HistoricalNetSyncEventsReceived, HistoricalNetSyncStart,
    KeyshareCreated, NetReady, OperatorActivationChanged, OutgoingSyncRequested,
    PartyProofsToVerify, PkAggregationProofPending, PkAggregationProofRequest,
    PkAggregationProofSigned, PkBfvProofResponse, PkGenerationProofRequest,
    PkGenerationProofSigned, PlaintextAggregated, PlaintextOutputPublished, Proof,
    ProofFailureAccusation, ProofPayload, ProofType, ProofVerificationFailed,
    ProofVerificationPassed, PublicKeyAggregated, PublishDocumentRequested, Seed,
    ShareComputationProofRequest, ShareDecryptionProofPending, ShareEncryptionProofRequest,
    ShareVerificationComplete, ShareVerificationDispatched, Shutdown, SignedProofFailed,
    SignedProofPayload, SlashExecuted, SyncEffect, SyncEnded, TestEvent, ThresholdShare,
    ThresholdShareCollectionFailed, ThresholdShareCreated, ThresholdShareDecryptionProofRequest,
    ThresholdSharePending, TicketBalanceUpdated, TicketGenerated, TicketId, TicketSubmitted,
    Unsequenced, VerificationKind, ZkResponse,
};
use e3_events::{AggregateId, EventStoreQueryBy, SeqAgg};
use e3_fhe_params::BfvPreset;
use e3_trbfv::shares::BfvEncryptedShares;
use e3_utils::actix::channel as actix_toolbox;
use e3_utils::ArcBytes;
use e3_zk_helpers::{computation::DkgInputType, CiphernodesCommitteeSize};
use std::collections::BTreeSet;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

fn dummy_proof(circuit: CircuitName) -> Proof {
    Proof::new(
        circuit,
        ArcBytes::from_bytes(&[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10,
        ]),
        ArcBytes::from_bytes(&[
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
            0x1f, 0x20,
        ]),
    )
}

fn dummy_signed_proof_payload(e3_id: E3id, proof_type: ProofType) -> SignedProofPayload {
    SignedProofPayload {
        payload: ProofPayload {
            e3_id,
            proof_type,
            proof: dummy_proof(CircuitName::PkBfv),
        },
        signature: ArcBytes::from_bytes(&[
            0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d,
            0x3e, 0x3f, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b,
            0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59,
            0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67,
            0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f,
        ]),
    }
}

fn dummy_accusation_vote(e3_id: E3id, voter: Address, agrees: bool) -> AccusationVote {
    AccusationVote {
        e3_id,
        accusation_id: [0xaa; 32],
        voter,
        agrees,
        data_hash: [0xbb; 32],
        signature: ArcBytes::from_bytes(&[0xcc; 64]),
    }
}

fn dummy_sensitive_bytes(data: &[u8]) -> SensitiveBytes {
    SensitiveBytes::from_encrypted(data)
}

fn dummy_threshold_share() -> ThresholdShare {
    ThresholdShare {
        party_id: 1,
        pk_share: ArcBytes::from_bytes(&[0x11; 32]),
        sk_sss: BfvEncryptedShares::default(),
        esi_sss: vec![BfvEncryptedShares::default()],
    }
}

fn dummy_pk_generation_proof_request() -> PkGenerationProofRequest {
    PkGenerationProofRequest::new(
        ArcBytes::from_bytes(&[0x22; 32]),
        dummy_sensitive_bytes(&[0x33; 32]),
        dummy_sensitive_bytes(&[0x44; 32]),
        dummy_sensitive_bytes(&[0x55; 32]),
        BfvPreset::InsecureThreshold512,
        CiphernodesCommitteeSize::Small,
    )
}

fn dummy_share_computation_proof_request() -> ShareComputationProofRequest {
    ShareComputationProofRequest {
        secret_raw: dummy_sensitive_bytes(&[0x66; 32]),
        secret_sss_raw: dummy_sensitive_bytes(&[0x77; 32]),
        dkg_input_type: DkgInputType::SecretKey,
        params_preset: BfvPreset::InsecureThreshold512,
        committee_size: CiphernodesCommitteeSize::Small,
    }
}

fn dummy_share_encryption_proof_request() -> ShareEncryptionProofRequest {
    ShareEncryptionProofRequest {
        share_row_raw: dummy_sensitive_bytes(&[0x88; 32]),
        ciphertext_raw: ArcBytes::from_bytes(&[0x99; 64]),
        recipient_pk_raw: ArcBytes::from_bytes(&[0xaa; 32]),
        u_rns_raw: dummy_sensitive_bytes(&[0xbb; 32]),
        e0_rns_raw: dummy_sensitive_bytes(&[0xcc; 32]),
        e1_rns_raw: dummy_sensitive_bytes(&[0xdd; 32]),
        dkg_input_type: DkgInputType::SecretKey,
        params_preset: BfvPreset::InsecureThreshold512,
        committee_size: CiphernodesCommitteeSize::Small,
        recipient_party_id: 2,
        row_index: 0,
        esi_index: 0,
    }
}

fn dummy_dkg_share_decryption_proof_request() -> DkgShareDecryptionProofRequest {
    DkgShareDecryptionProofRequest {
        sk_bfv: dummy_sensitive_bytes(&[0xee; 32]),
        honest_ciphertexts_raw: vec![ArcBytes::from_bytes(&[0xff; 64])],
        num_honest_parties: 3,
        num_moduli: 2,
        dkg_input_type: DkgInputType::SecretKey,
        params_preset: BfvPreset::InsecureThreshold512,
    }
}

fn dummy_threshold_share_decryption_proof_request() -> ThresholdShareDecryptionProofRequest {
    ThresholdShareDecryptionProofRequest {
        ciphertext_bytes: vec![ArcBytes::from_bytes(&[0x11; 64])],
        aggregated_pk_bytes: ArcBytes::from_bytes(&[0x22; 32]),
        sk_poly_sum: dummy_sensitive_bytes(&[0x33; 64]),
        es_poly_sum: vec![dummy_sensitive_bytes(&[0x44; 64])],
        d_share_bytes: vec![ArcBytes::from_bytes(&[0x55; 32])],
        params_preset: BfvPreset::InsecureThreshold512,
        proof_aggregation_enabled: true,
    }
}

fn dummy_pk_aggregation_proof_request() -> PkAggregationProofRequest {
    PkAggregationProofRequest {
        keyshare_bytes: vec![ArcBytes::from_bytes(&[0x66; 32])],
        aggregated_pk_bytes: ArcBytes::from_bytes(&[0x77; 32]),
        params_preset: BfvPreset::InsecureThreshold512,
        committee_n: 5,
        committee_h: 3,
        committee_threshold: 3,
    }
}

fn dummy_decrypted_shares_aggregation_proof_request() -> DecryptedSharesAggregationProofRequest {
    DecryptedSharesAggregationProofRequest {
        d_share_polys: vec![(1, vec![ArcBytes::from_bytes(&[0x88; 64])])],
        plaintext: vec![ArcBytes::from_bytes(&[0x99; 32])],
        params_preset: BfvPreset::InsecureThreshold512,
        threshold_m: 3,
        threshold_n: 5,
    }
}

#[derive(Subcommand, Clone, Debug)]
pub enum EventsCommands {
    /// Query events
    Query {
        /// Aggregate ID - will default to 0
        #[arg(long)]
        agg: Option<usize>,

        /// Sequence to read from will read from 0 if absent
        #[arg(long)]
        since: Option<u64>,

        /// Max limit to read at a time. If this is greater than the internal limit the internal
        /// limit will be respected.
        #[arg(long)]
        limit: Option<u64>,
    },
}

pub async fn execute(out: Console, command: EventsCommands, config: &AppConfig) -> Result<()> {
    match command {
        EventsCommands::Query { agg, since, limit } => {
            query_events(out, config, agg, since, limit).await?
        }
    }
    Ok(())
}

async fn query_events(
    out: Console,
    config: &AppConfig,
    aggregate: Option<usize>,
    since: Option<u64>,
    limit: Option<u64>,
) -> Result<()> {
    let eventstore = get_eventstore_reader(config)?;
    let events = fetch_events(eventstore, aggregate, since, limit).await?;
    print_events(out, events)?;
    Ok(())
}

async fn fetch_events(
    eventstore: EventStoreReader,
    aggregate: Option<usize>,
    since: Option<u64>,
    limit: Option<u64>,
) -> Result<Vec<EnclaveEvent>> {
    let aggregate = aggregate.unwrap_or(0);
    let since = since.unwrap_or(0);
    let limit = limit.unwrap_or(10);
    let (addr, rx) = actix_toolbox::oneshot::<EventStoreQueryResponse>();

    let msg = EventStoreQueryBy::<SeqAgg>::new(
        CorrelationId::new(),
        HashMap::from([(AggregateId::new(aggregate), since)]),
        addr,
    )
    .with_limit(limit);

    eventstore.seq().do_send(msg);
    let events = rx.await?.into_events();

    Ok(events)
}

fn print_events(out: Console, events: Vec<EnclaveEvent>) -> Result<()> {
    for event in events {
        log!(out, "{}", serde_json::to_string(&event)?);
    }
    Ok(())
}

/// This just prints fake events to ensure serialization works
async fn query_events_fake(aggregate: u64, since: u64, limit: u64) -> Result<()> {
    tracing::info!(
        "Querying events: aggregate={}, since={}, limit={}",
        aggregate,
        since,
        limit
    );

    let event1 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::PublicKeyAggregated(PublicKeyAggregated {
            pubkey: ArcBytes::from_bytes(&[
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
                0x77, 0x88,
            ]),
            e3_id: E3id::new("test1", 1),
            nodes: vec![
                "0x1234567890123456789012345678901234567890".to_string(),
                "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd".to_string(),
            ]
            .into(),
            pk_aggregation_proof: Some(dummy_proof(CircuitName::PkAggregation)),
            dkg_aggregated_proof: Some(dummy_proof(CircuitName::Fold)),
        }),
        None,
        1700000000000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(1);

    let event2 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::EncryptionKeyCreated(EncryptionKeyCreated {
            e3_id: E3id::new("test2", 1),
            key: Arc::new(EncryptionKey::new(
                42,
                ArcBytes::from_bytes(&[
                    0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0x0a, 0xbc, 0xde, 0xf1, 0x23,
                    0x45, 0x67, 0x89,
                ]),
            )),
            external: false,
        }),
        None,
        1700000001000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(2);

    let event3 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::E3Requested(E3Requested {
            e3_id: E3id::new("test3", 1),
            threshold_m: 3,
            threshold_n: 5,
            seed: Seed([
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
                0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
                0x1d, 0x1e, 0x1f, 0x20,
            ]),
            error_size: ArcBytes::from_bytes(&[
                0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0xDE, 0xAD, 0xC0, 0xDE,
            ]),
            esi_per_ct: 2,
            params: ArcBytes::from_bytes(&[
                0xBE, 0xEF, 0xFA, 0xCE, 0xDE, 0xAD, 0xFE, 0xED, 0xFE, 0xED, 0xCA, 0xFE,
            ]),
            proof_aggregation_enabled: true,
        }),
        None,
        1700000002000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(3);

    let event4 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::KeyshareCreated(KeyshareCreated {
            pubkey: ArcBytes::from_bytes(&[
                0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
                0xff, 0x00,
            ]),
            e3_id: E3id::new("test4", 1),
            node: "0xabcd1234abcd1234abcd1234abcd1234abcd1234".to_string(),
            signed_pk_generation_proof: Some(dummy_signed_proof_payload(
                E3id::new("test4", 1),
                ProofType::C1PkGeneration,
            )),
        }),
        None,
        1700000003000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(4);

    let event5 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::CiphertextOutputPublished(CiphertextOutputPublished {
            e3_id: E3id::new("test5", 1),
            ciphertext_output: vec![
                ArcBytes::from_bytes(&[
                    0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
                ]),
                ArcBytes::from_bytes(&[
                    0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
                ]),
            ],
        }),
        None,
        1700000004000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(5);

    let event6 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::DecryptionshareCreated(DecryptionshareCreated {
            party_id: 2,
            decryption_share: vec![ArcBytes::from_bytes(&[
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
            ])],
            e3_id: E3id::new("test6", 1),
            node: "0xfedc9876543210fedc9876543210fedc9876543210".to_string(),
            signed_decryption_proofs: vec![dummy_signed_proof_payload(
                E3id::new("test6", 1),
                ProofType::C6ThresholdShareDecryption,
            )],
            wrapped_proofs: vec![dummy_proof(CircuitName::ThresholdShareDecryption)],
        }),
        None,
        1700000005000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(6);

    let event7 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::PlaintextAggregated(PlaintextAggregated {
            e3_id: E3id::new("test7", 1),
            decrypted_output: vec![ArcBytes::from_bytes(&[
                0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00,
            ])],
            aggregation_proofs: vec![dummy_proof(CircuitName::DecryptedSharesAggregation)],
            c6_aggregated_proof: Some(dummy_proof(CircuitName::ThresholdShareDecryption)),
        }),
        None,
        1700000006000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(7);

    let event8 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::EnclaveError(EnclaveError {
            err_type: EType::Computation,
            message: "Computation failed: overflow detected in batch processing".to_string(),
        }),
        None,
        1700000007000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(8);

    let event9 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::TestEvent(TestEvent {
            msg: "Test message from CLI with full data".to_string(),
            entropy: 42,
            e3_id: Some(E3id::new("test9", 1)),
        }),
        None,
        1700000008000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(9);

    let event10 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::ConfigurationUpdated(ConfigurationUpdated {
            parameter: "max_committee_size".to_string(),
            old_value: alloy::primitives::U256::from(10),
            new_value: alloy::primitives::U256::from(20),
            chain_id: 1,
        }),
        None,
        1700000009000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(10);

    let accuser: Address = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        .parse()
        .unwrap();
    let accused: Address = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        .parse()
        .unwrap();

    let event11 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::AccusationQuorumReached(AccusationQuorumReached {
            e3_id: E3id::new("test11", 1),
            accuser,
            accused,
            proof_type: ProofType::C6ThresholdShareDecryption,
            votes_for: vec![
                dummy_accusation_vote(E3id::new("test11", 1), accuser, true),
                dummy_accusation_vote(
                    E3id::new("test11", 1),
                    "0xcccccccccccccccccccccccccccccccccccccccc"
                        .parse()
                        .unwrap(),
                    true,
                ),
            ],
            votes_against: vec![],
            outcome: AccusationOutcome::AccusedFaulted,
        }),
        None,
        1700000010000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(11);

    let event12 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::CiphernodeSelected(CiphernodeSelected {
            e3_id: E3id::new("test12", 1),
            threshold_m: 3,
            threshold_n: 5,
            seed: Seed([0x21; 32]),
            error_size: ArcBytes::from_bytes(&[0xFE; 16]),
            esi_per_ct: 4,
            params: ArcBytes::from_bytes(&[0xCA; 32]),
            party_id: 2,
        }),
        None,
        1700000011000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(12);

    let event13 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::CommitteePublished(CommitteePublished {
            e3_id: E3id::new("test13", 1),
            nodes: vec![
                "0x1111111111111111111111111111111111111111".to_string(),
                "0x2222222222222222222222222222222222222222".to_string(),
                "0x3333333333333333333333333333333333333333".to_string(),
            ],
            public_key: ArcBytes::from_bytes(&[0xAB; 64]),
            proof: ArcBytes::from_bytes(&[0xAB; 64]),
        }),
        None,
        1700000012000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(13);

    let event14 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::TicketGenerated(TicketGenerated {
            e3_id: E3id::new("test14", 1),
            ticket_id: TicketId::Score(42),
            node: "0x4444444444444444444444444444444444444444".to_string(),
        }),
        None,
        1700000013000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(14);

    let event15 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::TicketBalanceUpdated(TicketBalanceUpdated {
            operator: "0x5555555555555555555555555555555555555555".to_string(),
            delta: I256::MIN,
            new_balance: U256::from(900),
            reason: FixedBytes::from([0x66; 32]),
            chain_id: 1,
        }),
        None,
        1700000014000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(15);

    let event16 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::E3Failed(E3Failed {
            e3_id: E3id::new("test16", 1),
            failed_at_stage: E3Stage::Failed,
            reason: FailureReason::ComputeTimeout,
        }),
        None,
        1700000015000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(16);

    let event17 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::CiphernodeAdded(CiphernodeAdded {
            address: "0x7777777777777777777777777777777777777777".to_string(),
            index: 5,
            num_nodes: 10,
            chain_id: 1,
        }),
        None,
        1700000016000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(17);

    let accused_addr: Address = "0x8888888888888888888888888888888888888888"
        .parse()
        .unwrap();

    let event18 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::ProofVerificationFailed(ProofVerificationFailed {
            e3_id: E3id::new("test18", 1),
            accused_party_id: 3,
            accused_address: accused_addr,
            proof_type: ProofType::C1PkGeneration,
            data_hash: [0x99; 32],
            signed_payload: dummy_signed_proof_payload(
                E3id::new("test18", 1),
                ProofType::C1PkGeneration,
            ),
        }),
        None,
        1700000017000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(18);

    let event19 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::EnclaveError(EnclaveError {
            err_type: EType::KeyGeneration,
            message: "DKG timeout: insufficient shares received from peers".to_string(),
        }),
        None,
        1700000018000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(19);

    let event20 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::AccusationVote(AccusationVote {
            e3_id: E3id::new("test20", 1),
            accusation_id: [0xaa; 32],
            voter: "0x9999999999999999999999999999999999999999"
                .parse()
                .unwrap(),
            agrees: true,
            data_hash: [0xbb; 32],
            signature: ArcBytes::from_bytes(&[0xcc; 64]),
        }),
        None,
        1700000019000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(20);

    // Missing event types start here (event21 onwards)
    let event21 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::ProofFailureAccusation(ProofFailureAccusation {
            e3_id: E3id::new("test21", 1),
            accuser: "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .parse()
                .unwrap(),
            accused: "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                .parse()
                .unwrap(),
            accused_party_id: 2,
            proof_type: ProofType::C3aSkShareEncryption,
            data_hash: [0xdd; 32],
            signed_payload: Some(dummy_signed_proof_payload(
                E3id::new("test21", 1),
                ProofType::C3aSkShareEncryption,
            )),
            signature: ArcBytes::from_bytes(&[0xee; 64]),
        }),
        None,
        1700000020000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(21);

    let event22 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::ProofVerificationPassed(ProofVerificationPassed {
            e3_id: E3id::new("test22", 1),
            party_id: 1,
            address: "0x1111111111111111111111111111111111111111"
                .parse()
                .unwrap(),
            proof_type: ProofType::C1PkGeneration,
            data_hash: [0x22; 32],
        }),
        None,
        1700000021000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(22);

    let event23 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::DecryptionKeyShared(DecryptionKeyShared {
            e3_id: E3id::new("test23", 1),
            party_id: 1,
            node: "0x3333333333333333333333333333333333333333".to_string(),
            sk_poly_sum: ArcBytes::from_bytes(&[0x44; 32]),
            es_poly_sum: vec![ArcBytes::from_bytes(&[0x55; 32])],
            signed_sk_decryption_proof: dummy_signed_proof_payload(
                E3id::new("test23", 1),
                ProofType::C4DkgShareDecryption,
            ),
            signed_e_sm_decryption_proofs: vec![dummy_signed_proof_payload(
                E3id::new("test23", 1),
                ProofType::C4DkgShareDecryption,
            )],
            external: false,
        }),
        None,
        1700000022000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(23);

    let event24 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::PublishDocumentRequested(PublishDocumentRequested {
            meta: DocumentMeta::new(
                E3id::new("test24", 1),
                DocumentKind::TrBFV,
                vec![],
                Some(Utc::now()),
            ),
            value: ArcBytes::from_bytes(&[0x66; 64]),
        }),
        None,
        1700000023000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(24);

    let event25 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::PkGenerationProofSigned(PkGenerationProofSigned {
            e3_id: E3id::new("test25", 1),
            party_id: 1,
            signed_proof: dummy_signed_proof_payload(
                E3id::new("test25", 1),
                ProofType::C1PkGeneration,
            ),
        }),
        None,
        1700000024000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(25);

    let event26 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::DkgProofSigned(DkgProofSigned {
            e3_id: E3id::new("test26", 1),
            party_id: 1,
            signed_proof: dummy_signed_proof_payload(
                E3id::new("test26", 1),
                ProofType::C2aSkShareComputation,
            ),
        }),
        None,
        1700000025000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(26);

    let event27 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::E3RequestComplete(E3RequestComplete {
            e3_id: E3id::new("test27", 1),
        }),
        None,
        1700000026000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(27);

    let event28 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::E3StageChanged(E3StageChanged {
            e3_id: E3id::new("test28", 1),
            previous_stage: E3Stage::CiphertextReady,
            new_stage: E3Stage::Complete,
        }),
        None,
        1700000027000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(28);

    let event29 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::CiphernodeRemoved(CiphernodeRemoved {
            address: "0x9999999999999999999999999999999999999999".to_string(),
            index: 3,
            num_nodes: 9,
            chain_id: 1,
        }),
        None,
        1700000028000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(29);

    let event30 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::OperatorActivationChanged(OperatorActivationChanged {
            operator: "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            active: true,
            chain_id: 1,
        }),
        None,
        1700000029000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(30);

    let event31 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::CommitteeRequested(CommitteeRequested {
            e3_id: E3id::new("test31", 1),
            seed: Seed([0xaa; 32]),
            threshold: [3, 5],
            request_block: 100,
            committee_deadline: 200,
            chain_id: 1,
        }),
        None,
        1700000030000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(31);

    let event32 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::CommitteeFinalizeRequested(CommitteeFinalizeRequested {
            e3_id: E3id::new("test32", 1),
        }),
        None,
        1700000031000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(32);

    let event33 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::CommitteeFinalized(CommitteeFinalized {
            e3_id: E3id::new("test33", 1),
            committee: vec![
                "0x1111111111111111111111111111111111111111".to_string(),
                "0x2222222222222222222222222222222222222222".to_string(),
            ],
            chain_id: 1,
        }),
        None,
        1700000032000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(33);

    let event34 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::TicketSubmitted(TicketSubmitted {
            e3_id: E3id::new("test34", 1),
            node: "0x4444444444444444444444444444444444444444".to_string(),
            ticket_id: 123,
            score: "0.95".to_string(),
            chain_id: 1,
        }),
        None,
        1700000033000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(34);

    let event35 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::PlaintextOutputPublished(PlaintextOutputPublished {
            e3_id: E3id::new("test35", 1),
            plaintext_output: ArcBytes::from_bytes(&[0x55; 32]),
            proof: ArcBytes::from_bytes(&[0x66; 64]),
        }),
        None,
        1700000034000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(35);

    let event36 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::Shutdown(Shutdown),
        None,
        1700000035000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(36);

    let event37 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::DocumentReceived(DocumentReceived {
            meta: DocumentMeta::new(
                E3id::new("test37", 1),
                DocumentKind::TrBFV,
                vec![],
                Some(Utc::now()),
            ),
            value: ArcBytes::from_bytes(&[0x66; 64]),
        }),
        None,
        1700000036000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(37);

    let event38 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::EncryptionKeyPending(EncryptionKeyPending {
            e3_id: E3id::new("test40", 1),
            key: Arc::new(EncryptionKey::new(42, ArcBytes::from_bytes(&[0x33; 16]))),
            params_preset: e3_fhe_params::BfvPreset::InsecureThreshold512,
        }),
        None,
        1700000039000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(38);

    let event39 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::EncryptionKeyReceived(EncryptionKeyReceived {
            e3_id: E3id::new("test41", 1),
            key: Arc::new(EncryptionKey::new(42, ArcBytes::from_bytes(&[0x44; 16]))),
        }),
        None,
        1700000040000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(39);

    let event40 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::EncryptionKeyCollectionFailed(EncryptionKeyCollectionFailed {
            e3_id: E3id::new("test42", 1),
            reason: "Timeout waiting for encryption keys".to_string(),
            missing_parties: vec![2, 4],
        }),
        None,
        1700000041000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(40);

    let event41 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::ThresholdShareCollectionFailed(ThresholdShareCollectionFailed {
            e3_id: E3id::new("test43", 1),
            reason: "Insufficient shares received".to_string(),
            missing_parties: vec![3, 5],
        }),
        None,
        1700000042000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(41);

    let event42 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::SignedProofFailed(SignedProofFailed {
            e3_id: E3id::new("test47", 1),
            faulting_node: "0x7777777777777777777777777777777777777777"
                .parse()
                .unwrap(),
            proof_type: ProofType::C1PkGeneration,
            signed_payload: dummy_signed_proof_payload(
                E3id::new("test47", 1),
                ProofType::C1PkGeneration,
            ),
        }),
        None,
        1700000046000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(42);

    let event43 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::SlashExecuted(SlashExecuted {
            e3_id: E3id::new("test51", 1),
            proposal_id: 12345,
            operator: "0x9999999999999999999999999999999999999999"
                .parse()
                .unwrap(),
            reason: [0xaa; 32],
            ticket_amount: 1000,
            license_amount: 500,
        }),
        None,
        1700000050000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(43);

    let event44 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::CommitteeMemberExpelled(CommitteeMemberExpelled {
            e3_id: E3id::new("test52", 1),
            node: "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .parse()
                .unwrap(),
            reason: [0xbb; 32],
            active_count_after: 9,
            party_id: Some(2),
        }),
        None,
        1700000051000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(44);

    let event45 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::OutgoingSyncRequested(OutgoingSyncRequested {
            since: vec![(AggregateId::new(0), 1700000000000_u128)],
        }),
        None,
        1700000052000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(45);

    let event46 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::HistoricalEvmSyncStart(HistoricalEvmSyncStart {
            evm_config: EvmEventConfig::from_config(
                [(1, EvmEventConfigChain::new(100))]
                    .into_iter()
                    .collect::<BTreeMap<_, _>>(),
            ),
            sender: None,
        }),
        None,
        1700000053000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(46);

    let event47 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::HistoricalNetSyncStart(HistoricalNetSyncStart {
            since: BTreeMap::from([(AggregateId::new(0), 1700000000000_u128)]),
        }),
        None,
        1700000054000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(47);

    let event48 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::HistoricalNetSyncEventsReceived(HistoricalNetSyncEventsReceived {
            events: vec![],
        }),
        None,
        1700000055000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(48);

    let event49 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::SyncEffect(SyncEffect::new()),
        None,
        1700000056000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(49);

    let event50 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::SyncEnded(SyncEnded::new()),
        None,
        1700000057000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(50);

    let event51 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::EffectsEnabled(EffectsEnabled::new()),
        None,
        1700000058000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(51);

    let event52 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::NetReady(NetReady::new()),
        None,
        1700000059000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(52);

    let event53 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::DecryptionShareProofSigned(DecryptionShareProofSigned {
            e3_id: E3id::new("test61", 1),
        }),
        None,
        1700000060000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(53);

    let event54 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::AggregationProofSigned(AggregationProofSigned {
            e3_id: E3id::new("test66", 1),
            signed_proofs: vec![dummy_signed_proof_payload(
                E3id::new("test66", 1),
                ProofType::C7DecryptedSharesAggregation,
            )],
        }),
        None,
        1700000065000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(54);

    let event55 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::DKGInnerProofReady(DKGInnerProofReady {
            e3_id: E3id::new("test67", 1),
            party_id: 1,
            wrapped_proof: dummy_proof(CircuitName::Fold),
            seq: 0,
        }),
        None,
        1700000066000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(55);

    let event56 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::DKGRecursiveAggregationComplete(DKGRecursiveAggregationComplete {
            e3_id: E3id::new("test68", 1),
            party_id: 1,
            aggregated_proof: Some(dummy_proof(CircuitName::Fold)),
        }),
        None,
        1700000067000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(56);

    let event57 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::ThresholdSharePending(ThresholdSharePending {
            e3_id: E3id::new("test57", 1),
            full_share: Arc::new(dummy_threshold_share()),
            proof_request: dummy_pk_generation_proof_request(),
            sk_share_computation_request: dummy_share_computation_proof_request(),
            e_sm_share_computation_request: dummy_share_computation_proof_request(),
            sk_share_encryption_requests: vec![dummy_share_encryption_proof_request()],
            e_sm_share_encryption_requests: vec![dummy_share_encryption_proof_request()],
            recipient_party_ids: vec![1, 2, 3, 4, 5],
            proof_aggregation_enabled: true,
        }),
        None,
        1700000070000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(57);

    let event58 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::ThresholdShareCreated(ThresholdShareCreated {
            e3_id: E3id::new("test58", 1),
            share: Arc::new(dummy_threshold_share()),
            target_party_id: 1,
            external: false,
            signed_c2a_proof: Some(dummy_signed_proof_payload(
                E3id::new("test58", 1),
                ProofType::C2aSkShareComputation,
            )),
            signed_c2b_proof: None,
            signed_c3a_proofs: vec![],
            signed_c3b_proofs: vec![],
        }),
        None,
        1700000071000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(58);

    let event59 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::ShareDecryptionProofPending(ShareDecryptionProofPending {
            e3_id: E3id::new("test59", 1),
            party_id: 1,
            node: "0x1111111111111111111111111111111111111111".to_string(),
            decryption_share: vec![ArcBytes::from_bytes(&[0x11; 64])],
            proof_request: dummy_threshold_share_decryption_proof_request(),
        }),
        None,
        1700000072000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(59);

    let event60 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::DecryptionShareProofsPending(DecryptionShareProofsPending {
            e3_id: E3id::new("test60", 1),
            party_id: 1,
            node: "0x2222222222222222222222222222222222222222".to_string(),
            sk_poly_sum: ArcBytes::from_bytes(&[0x33; 32]),
            es_poly_sum: vec![ArcBytes::from_bytes(&[0x44; 32])],
            sk_request: dummy_dkg_share_decryption_proof_request(),
            esm_requests: vec![dummy_dkg_share_decryption_proof_request()],
        }),
        None,
        1700000073000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(60);

    let event61 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::PkAggregationProofPending(PkAggregationProofPending {
            e3_id: E3id::new("test61", 1),
            proof_request: dummy_pk_aggregation_proof_request(),
            public_key: ArcBytes::from_bytes(&[0x55; 32]),
            nodes: vec![
                "0x1111111111111111111111111111111111111111".to_string(),
                "0x2222222222222222222222222222222222222222".to_string(),
            ]
            .into(),
        }),
        None,
        1700000074000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(61);

    let event62 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::AggregationProofPending(AggregationProofPending {
            e3_id: E3id::new("test62", 1),
            proof_request: dummy_decrypted_shares_aggregation_proof_request(),
            plaintext: vec![ArcBytes::from_bytes(&[0x66; 32])],
            shares: vec![(1, vec![ArcBytes::from_bytes(&[0x77; 32])])],
        }),
        None,
        1700000075000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(62);

    let event63 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::ShareVerificationDispatched(ShareVerificationDispatched {
            e3_id: E3id::new("test63", 1),
            kind: VerificationKind::ShareProofs,
            share_proofs: vec![PartyProofsToVerify {
                sender_party_id: 2,
                signed_proofs: vec![dummy_signed_proof_payload(
                    E3id::new("test63", 1),
                    ProofType::C2aSkShareComputation,
                )],
            }],
            decryption_proofs: vec![],
            pre_dishonest: BTreeSet::new(),
        }),
        None,
        1700000076000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(63);

    let event64 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::ShareVerificationComplete(ShareVerificationComplete {
            e3_id: E3id::new("test64", 1),
            kind: VerificationKind::ShareProofs,
            dishonest_parties: BTreeSet::from([3, 5]),
        }),
        None,
        1700000077000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(64);

    let event65 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::ComputeResponse(ComputeResponse {
            response: ComputeResponseKind::Zk(ZkResponse::PkBfv(PkBfvProofResponse {
                proof: dummy_proof(CircuitName::PkBfv),
                wrapped_proof: dummy_proof(CircuitName::PkBfv),
            })),
            correlation_id: CorrelationId::new(),
            e3_id: E3id::new("test65", 1),
        }),
        None,
        1700000078000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(65);

    let event66 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::PkAggregationProofSigned(PkAggregationProofSigned {
            e3_id: E3id::new("test66", 1),
            signed_proof: dummy_signed_proof_payload(
                E3id::new("test66", 1),
                ProofType::C5PkAggregation,
            ),
        }),
        None,
        1700000079000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(66);

    for event in [
        event1, event2, event3, event4, event5, event6, event7, event8, event9, event10, event11,
        event12, event13, event14, event15, event16, event17, event18, event19, event20, event21,
        event22, event23, event24, event25, event26, event27, event28, event29, event30, event31,
        event32, event33, event34, event35, event36, event37, event38, event39, event40, event41,
        event42, event43, event44, event45, event46, event47, event48, event49, event50, event51,
        event52, event53, event54, event55, event56, event57, event58, event59, event60, event61,
        event62, event63, event64, event65, event66,
    ] {
        println!("{}", serde_json::to_string(&event)?);
    }

    Ok(())
}
