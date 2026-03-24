// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::primitives::{Address, FixedBytes, I256, U256};
use anyhow::Result;
use clap::Subcommand;
use e3_events::{
    AccusationOutcome, AccusationQuorumReached, AccusationVote, CiphernodeAdded,
    CiphernodeSelected, CiphertextOutputPublished, CircuitName, CommitteePublished,
    ConfigurationUpdated, DecryptionshareCreated, E3Failed, E3Requested, E3Stage, E3id, EType,
    EnclaveError, EnclaveEvent, EnclaveEventData, EncryptionKey, EncryptionKeyCreated,
    EventConstructorWithTimestamp, FailureReason, KeyshareCreated, PlaintextAggregated, Proof,
    ProofPayload, ProofType, ProofVerificationFailed, PublicKeyAggregated, Seed,
    SignedProofPayload, TestEvent, TicketBalanceUpdated, TicketGenerated, TicketId, Unsequenced,
};
use e3_utils::ArcBytes;
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

#[derive(Subcommand, Clone, Debug)]
pub enum EventsCommands {
    /// Query events
    Query {
        /// Aggregate ID - will default to 0
        #[arg(long)]
        agg: Option<u64>,

        /// Sequence to read from will read from 0 if absent
        #[arg(long)]
        since: Option<u64>,

        /// Max limit to read at a time. If this is greater than the internal limit the internal
        /// limit will be respected.
        #[arg(long)]
        limit: Option<u64>,
    },
}

pub async fn execute(command: EventsCommands) -> Result<()> {
    match command {
        EventsCommands::Query { agg, since, limit } => {
            query_events(agg.unwrap_or(0), since.unwrap_or(0), limit.unwrap_or(10)).await?;
        }
    }
    Ok(())
}

async fn query_events(aggregate: u64, since: u64, limit: u64) -> Result<()> {
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

    for event in [
        event1, event2, event3, event4, event5, event6, event7, event8, event9, event10, event11,
        event12, event13, event14, event15, event16, event17, event18, event19, event20,
    ] {
        println!("{}", serde_json::to_string(&event)?);
    }
    Ok(())
}
