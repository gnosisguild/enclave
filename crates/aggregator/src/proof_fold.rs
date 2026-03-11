// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_events::{
    prelude::*, BusHandle, ComputeRequest, CorrelationId, E3id, EventContext, Proof, Sequenced,
    ZkRequest,
};
use tracing::{error, info};

/// Manages the state of a sequential `FoldProofs` operation.
///
/// Takes an ordered list of proofs and folds them pairwise via `ZkRequest::FoldProofs`
/// until a single aggregated proof remains. The caller owns the struct and checks
/// `result` (or calls `is_complete`) to know when folding is done.
///
/// Serialization support enables persistence during VerifyingC1/GeneratingC5Proof for restart recovery.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ProofFoldState {
    correlation: Option<CorrelationId>,
    accumulated: Option<Proof>,
    remaining: Vec<Proof>,
    /// Set when all fold steps have completed.
    pub result: Option<Proof>,
}

impl ProofFoldState {
    pub fn new() -> Self {
        ProofFoldState {
            correlation: None,
            accumulated: None,
            remaining: Vec::new(),
            result: None,
        }
    }

    /// Returns `true` if no fold has been initiated yet (idle / not started).
    pub fn is_idle(&self) -> bool {
        self.correlation.is_none()
            && self.accumulated.is_none()
            && self.remaining.is_empty()
            && self.result.is_none()
    }

    /// Begin folding `proofs` sequentially.
    ///
    /// - 0 proofs → `result` stays `None`
    /// - 1 proof  → `result` is set immediately, no ZK request dispatched
    /// - N proofs → first fold step dispatched; subsequent steps follow via `handle_response`
    pub fn start(
        &mut self,
        mut proofs: Vec<Proof>,
        label: &str,
        bus: &BusHandle,
        e3_id: &E3id,
        ec: &EventContext<Sequenced>,
    ) -> Result<()> {
        match proofs.len() {
            0 => {
                info!("{label}: no proofs to fold");
                Ok(())
            }
            1 => {
                info!("{label}: single proof — no fold needed");
                self.result = Some(proofs.remove(0));
                Ok(())
            }
            _ => {
                let first = proofs.remove(0);
                self.accumulated = Some(first);
                self.remaining = proofs;
                info!(
                    "{label}: starting fold ({} steps remaining)",
                    self.remaining.len()
                );
                self.advance(label, bus, e3_id, ec)
            }
        }
    }

    /// Handle a `FoldProofs` response. Returns `true` if `correlation_id` matched this fold.
    ///
    /// On match, either dispatches the next step or finalises `result`.
    pub fn handle_response(
        &mut self,
        correlation_id: &CorrelationId,
        proof: Proof,
        label: &str,
        bus: &BusHandle,
        e3_id: &E3id,
        ec: &EventContext<Sequenced>,
    ) -> Result<bool> {
        let Some(expected) = self.correlation else {
            return Ok(false);
        };
        if expected != *correlation_id {
            return Ok(false);
        }

        self.correlation = None;
        self.accumulated = Some(proof);

        info!(
            "{label}: fold step done ({} remaining)",
            self.remaining.len()
        );

        if self.remaining.is_empty() {
            self.result = self.accumulated.take();
        } else {
            self.advance(label, bus, e3_id, ec)?;
        }

        Ok(true)
    }

    fn advance(
        &mut self,
        label: &str,
        bus: &BusHandle,
        e3_id: &E3id,
        ec: &EventContext<Sequenced>,
    ) -> Result<()> {
        if self.correlation.is_some() {
            return Ok(());
        }

        let Some(acc) = self.accumulated.take() else {
            return Ok(());
        };

        let Some(next) = self.remaining.first().cloned() else {
            self.result = Some(acc);
            return Ok(());
        };
        self.remaining.remove(0);
        let target_evm = self.remaining.is_empty();

        let corr = CorrelationId::new();
        self.correlation = Some(corr);

        info!(
            "{label}: dispatching fold step ({} remaining, target_evm={})",
            self.remaining.len(),
            target_evm
        );

        if let Err(err) = bus.publish(
            ComputeRequest::zk(
                ZkRequest::FoldProofs {
                    proof1: acc,
                    proof2: next,
                    target_evm,
                },
                corr,
                e3_id.clone(),
            ),
            ec.clone(),
        ) {
            error!("{label}: failed to publish fold request: {err}");
            self.correlation = None;
        }

        Ok(())
    }
}
