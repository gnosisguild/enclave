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
    /// Total fold steps (for progress logging). Set when fold starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    total_steps: Option<usize>,
    /// Set when all fold steps have completed.
    pub result: Option<Proof>,
    /// `start` was called with zero proofs — folding is complete with no aggregate.
    #[serde(default)]
    pub fold_input_was_empty: bool,
}

impl ProofFoldState {
    pub fn new() -> Self {
        ProofFoldState {
            correlation: None,
            accumulated: None,
            remaining: Vec::new(),
            total_steps: None,
            result: None,
            fold_input_was_empty: false,
        }
    }

    /// Returns `true` if a fold step was dispatched but the in-flight proof was consumed
    /// and the response will never arrive (e.g. after a restart). The caller should reset
    /// and re-initiate the fold from the original proofs.
    pub fn needs_restart(&self) -> bool {
        self.correlation.is_some() && self.accumulated.is_none() && self.result.is_none()
    }

    /// Returns `true` if no fold has been initiated yet (idle / not started).
    pub fn is_idle(&self) -> bool {
        self.correlation.is_none()
            && self.accumulated.is_none()
            && self.remaining.is_empty()
            && self.total_steps.is_none()
            && self.result.is_none()
    }

    /// Begin folding `proofs` sequentially.
    ///
    /// - 0 proofs → `result` stays `None`, `fold_input_was_empty` is set (caller can treat fold as done)
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
        self.correlation = None;
        self.accumulated = None;
        self.remaining.clear();
        self.total_steps = None;
        self.result = None;
        self.fold_input_was_empty = false;

        match proofs.len() {
            0 => {
                info!("{label}: no proofs to fold");
                self.fold_input_was_empty = true;
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
                let total = self.remaining.len();
                self.total_steps = Some(total);
                info!(
                    "{label}: starting fold ({} steps total, {} proofs remaining)",
                    total,
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

        let step_done = self
            .total_steps
            .map(|t| t - self.remaining.len())
            .unwrap_or(0);
        info!(
            "{label}: fold step {}/{} done ({} remaining)",
            step_done,
            self.total_steps.unwrap_or(0),
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

        let step = self
            .total_steps
            .map(|t| t - self.remaining.len())
            .unwrap_or(0);
        info!(
            "{label}: dispatching fold step {}/{} ({} proofs remaining, target_evm={})",
            step,
            self.total_steps.unwrap_or(0),
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
