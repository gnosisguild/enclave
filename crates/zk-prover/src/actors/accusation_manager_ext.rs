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
use tracing::{error, info};

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

        // Parse committee addresses — all must be valid or we cannot start
        let mut committee_addresses: Vec<Address> = Vec::with_capacity(committee.len());
        for s in committee.iter() {
            match s.parse::<Address>() {
                Ok(addr) => committee_addresses.push(addr),
                Err(e) => {
                    error!(
                        "Failed to parse committee address {} — cannot start AccusationManager: {}",
                        s, e
                    );
                    return;
                }
            }
        }

        if committee_addresses.is_empty() {
            error!("No committee addresses — cannot start AccusationManager");
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
            meta.params_preset,
        );

        ctx.set_event_recipient("accusation_manager", Some(addr.into()));
    }

    /// Re-hydrates the `AccusationManager` after a node restart.
    ///
    /// Intentionally a no-op — `AccusationManager` is **ephemeral by design**:
    ///
    /// - Each instance is scoped to one E3 (created by [`AccusationManagerExtension::handle`]
    ///   when `CommitteeFinalized` is received) and holds only transient in-memory state
    ///   (pending accusations, buffered votes, verification caches).
    /// - On restart, all in-flight accusations are lost. This is an accepted trade-off:
    ///   every pending accusation has a finite vote timeout (default 5 min). If the node
    ///   restarts, the accusation would have timed out anyway. Other committee members
    ///   running their own independent `AccusationManager` instances will continue the
    ///   protocol unaffected.
    /// - A malicious node cannot exploit restart-induced state loss to prevent slashing:
    ///   restarting only loses *this node's* pending state — all other honest nodes still
    ///   independently verify, vote, and reach quorum without this node's participation
    ///   (as long as enough honest nodes remain to meet threshold M).
    async fn hydrate(&self, _ctx: &mut E3Context, _snapshot: &E3ContextSnapshot) -> Result<()> {
        Ok(())
    }
}
