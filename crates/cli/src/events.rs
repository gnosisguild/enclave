// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use clap::Subcommand;
use e3_events::{
    E3id, EnclaveEvent, EnclaveEventData, EncryptionKey, EncryptionKeyCreated,
    EventConstructorWithTimestamp, PublicKeyAggregated, Unsequenced,
};
use e3_utils::ArcBytes;
use std::sync::Arc;

#[derive(Subcommand, Clone, Debug)]
pub enum EventsCommands {
    /// Query events
    Query {
        #[arg(long)]
        aggregate: u64,

        #[arg(long)]
        since: u64,

        #[arg(long)]
        limit: u64,
    },
}

pub async fn execute(command: EventsCommands) -> Result<()> {
    match command {
        EventsCommands::Query {
            aggregate,
            since,
            limit,
        } => {
            query_events(aggregate, since, limit).await?;
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

    for event in [event1, event2] {
        println!("{}", serde_json::to_string(&event)?);
    }
    Ok(())
}
