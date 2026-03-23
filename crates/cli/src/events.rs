// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use clap::Subcommand;
use e3_events::{
    CiphertextOutputPublished, ConfigurationUpdated, DecryptionshareCreated, E3Requested, E3id,
    EType, EnclaveError, EnclaveEvent, EnclaveEventData, EncryptionKey, EncryptionKeyCreated,
    EventConstructorWithTimestamp, KeyshareCreated, PlaintextAggregated, PublicKeyAggregated, Seed,
    TestEvent, Unsequenced,
};
use e3_utils::ArcBytes;
use std::sync::Arc;

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
            pubkey: ArcBytes::from_bytes(&[0x12, 0x34, 0x56]),
            e3_id: E3id::new("test1", 1),
            nodes: Default::default(),
            pk_aggregation_proof: None,
            dkg_aggregated_proof: None,
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
                ArcBytes::from_bytes(&[0xab, 0xcd, 0xef]),
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
            seed: Seed([1u8; 32]),
            error_size: ArcBytes::from_bytes(&[0xDE, 0xAD]),
            esi_per_ct: 2,
            params: ArcBytes::from_bytes(&[0xBE, 0xEF]),
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
            pubkey: ArcBytes::from_bytes(&[0x11, 0x22, 0x33]),
            e3_id: E3id::new("test4", 1),
            node: "node_abc".to_string(),
            signed_pk_generation_proof: None,
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
                ArcBytes::from_bytes(&[0xAA, 0xBB, 0xCC]),
                ArcBytes::from_bytes(&[0xDD, 0xEE, 0xFF]),
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
            decryption_share: vec![ArcBytes::from_bytes(&[0x01, 0x02])],
            e3_id: E3id::new("test6", 1),
            node: "node_xyz".to_string(),
            signed_decryption_proofs: vec![],
            wrapped_proofs: vec![],
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
            decrypted_output: vec![ArcBytes::from_bytes(&[0x99, 0x88])],
            aggregation_proofs: vec![],
            c6_aggregated_proof: None,
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
            message: "Computation failed: overflow detected".to_string(),
        }),
        None,
        1700000007000_u128,
        None,
        e3_events::EventSource::Local,
    )
    .into_sequenced(8);

    let event9 = EnclaveEvent::<Unsequenced>::new_with_timestamp(
        EnclaveEventData::TestEvent(TestEvent {
            msg: "Test message from CLI".to_string(),
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

    for event in [
        event1, event2, event3, event4, event5, event6, event7, event8, event9, event10,
    ] {
        println!("{}", serde_json::to_string(&event)?);
    }
    Ok(())
}
