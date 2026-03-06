// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! E3Extension that wires up the [`AccusationManager`] per-E3 when the
//! committee is finalized.
//!
//! Listens for [`CommitteeFinalized`], reads `threshold_m` from [`E3Meta`],
//! parses committee addresses, and starts the actor with full context.

use crate::AccusationManager;
use alloy::primitives::Address;
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use async_trait::async_trait;
use e3_events::{BusHandle, CommitteeFinalized, EnclaveEvent, EnclaveEventData, Event};
use e3_request::{E3Context, E3ContextSnapshot, E3Extension, META_KEY};
use tracing::{error, info, warn};

pub struct AccusationManagerExtension {
    bus: BusHandle,
    signer: PrivateKeySigner,
}

impl AccusationManagerExtension {
    pub fn create(bus: &BusHandle, signer: PrivateKeySigner) -> Box<Self> {
        Box::new(Self {
            bus: bus.clone(),
            signer: signer.clone(),
        })
    }
}

#[async_trait]
impl E3Extension for AccusationManagerExtension {
    fn on_event(&self, ctx: &mut E3Context, evt: &EnclaveEvent) {
        let EnclaveEventData::CommitteeFinalized(data) = evt.get_data() else {
            return;
        };

        // Don't start twice
        if ctx.get_event_recipient("accusation_manager").is_some() {
            return;
        }

        let CommitteeFinalized {
            e3_id, committee, ..
        } = data.clone();

        // Parse committee addresses
        let committee_addresses: Vec<Address> = committee
            .iter()
            .filter_map(|s| match s.parse::<Address>() {
                Ok(addr) => Some(addr),
                Err(e) => {
                    warn!("Failed to parse committee address {}: {}", s, e);
                    None
                }
            })
            .collect();

        if committee_addresses.is_empty() {
            error!("No valid committee addresses — cannot start AccusationManager");
            return;
        }

        // Get threshold from meta
        let Some(meta) = ctx.get_dependency(META_KEY) else {
            error!("E3Meta not available — cannot start AccusationManager");
            return;
        };
        let threshold_m = meta.threshold_m;

        info!(
            "Starting AccusationManager for E3 {} with {} committee members, threshold={}",
            e3_id,
            committee_addresses.len(),
            threshold_m
        );

        let addr = AccusationManager::setup(
            &self.bus,
            e3_id,
            self.signer.clone(),
            committee_addresses,
            threshold_m,
        );

        ctx.set_event_recipient("accusation_manager", Some(addr.into()));
    }

    async fn hydrate(&self, _ctx: &mut E3Context, _snapshot: &E3ContextSnapshot) -> Result<()> {
        // AccusationManager is ephemeral — no state to hydrate.
        // On restart, in-flight accusations are lost (acceptable: they would
        // have timed out anyway). The actor will be re-created on the next
        // CommitteeFinalized.
        Ok(())
    }
}
