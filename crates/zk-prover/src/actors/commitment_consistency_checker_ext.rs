// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! E3Extension that wires up the [`CommitmentConsistencyChecker`] per-E3
//! when the committee is finalized.
//!
//! Follows the same lifecycle pattern as [`AccusationManagerExtension`]:
//! listens for [`CommitteeFinalized`], creates the actor, and registers it
//! in the [`E3Context`] so it receives routed events.

use super::commitment_consistency_checker::CommitmentConsistencyChecker;
use super::commitment_links;
use anyhow::Result;
use async_trait::async_trait;
use e3_events::{BusHandle, EnclaveEvent, EnclaveEventData, Event};
use e3_request::{E3Context, E3ContextSnapshot, E3Extension, META_KEY};
use tracing::{error, info};

pub struct CommitmentConsistencyCheckerExtension {
    bus: BusHandle,
}

impl CommitmentConsistencyCheckerExtension {
    pub fn create(bus: &BusHandle) -> Box<Self> {
        Box::new(Self { bus: bus.clone() })
    }
}

#[async_trait]
impl E3Extension for CommitmentConsistencyCheckerExtension {
    fn on_event(&self, ctx: &mut E3Context, evt: &EnclaveEvent) {
        let EnclaveEventData::CommitteeFinalized(data) = evt.get_data() else {
            return;
        };

        // Don't start twice
        if ctx
            .get_event_recipient("commitment_consistency_checker")
            .is_some()
        {
            return;
        }

        let e3_id = data.e3_id.clone();

        let Some(meta) = ctx.get_dependency(META_KEY) else {
            error!("E3Meta not available — cannot start CommitmentConsistencyChecker");
            return;
        };

        info!("Starting CommitmentConsistencyChecker for E3 {}", e3_id);

        let links = commitment_links::default_links(meta.params_preset);
        let addr = CommitmentConsistencyChecker::setup(&self.bus, e3_id, links);

        ctx.set_event_recipient("commitment_consistency_checker", Some(addr.into()));
    }

    /// Intentionally a no-op — the checker is ephemeral by design (same
    /// reasoning as [`AccusationManagerExtension::hydrate`]).
    async fn hydrate(&self, _ctx: &mut E3Context, _snapshot: &E3ContextSnapshot) -> Result<()> {
        Ok(())
    }
}
