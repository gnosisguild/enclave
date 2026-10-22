// SPDX-License-Identifier: LGPL-3.0-only

//! Ciphertext handling, C6 share calculation, and restart redrive.

use super::*;
use e3_events::FreshDecryptionRound;

impl ThresholdKeyshare {
    /// Handle the ciphertext output and begin local C6 decryption-share work.
    pub fn handle_ciphertext_output_published(
        &mut self,
        msg: TypedEvent<CiphertextOutputPublished>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();
        let ciphertext_output = msg.ciphertext_output;

        // A replayed ciphertext can resume work that this process has not issued.
        {
            let state = self.state.try_get()?;
            match &state.state {
                KeyshareState::Decrypting(_) => {
                    info!(
                        e3_id = %state.e3_id,
                        "CiphertextOutputPublished received while already Decrypting — resuming pending decryption-share work"
                    );
                    return self.issue_decryption_share_request(ec);
                }
                KeyshareState::GeneratingDecryptionProof(_) | KeyshareState::Completed(_) => {
                    info!(
                        e3_id = %state.e3_id,
                        state = %state.variant_name(),
                        "CiphertextOutputPublished received after decryption already in progress — ignoring"
                    );
                    return Ok(());
                }
                KeyshareState::ReadyForDecryption(_) => {
                    // Normal path — proceed with transition below.
                }
                other => {
                    warn!(
                        e3_id = %state.e3_id,
                        state = %other.variant_name(),
                        "CiphertextOutputPublished received in unexpected state — cannot process decryption"
                    );
                    return Ok(());
                }
            }
        }

        // Reject a missing key or a short honest set before the phase changes.
        let decryptors = {
            let state = self.state.try_get()?;
            party_index(state.party_id)?;
            canonical_decryptors(
                state.honest_parties.as_ref(),
                &state.expelled_parties,
                reconstruction_count(state.threshold_m)?,
            )?
        };

        // Set state to decrypting, storing ciphertext for later C6 proof generation
        self.state.try_mutate(&ec, |s| {
            use KeyshareState as K;

            let current: ReadyForDecryption = s.clone().try_into()?;

            let next = K::Decrypting(Decrypting {
                pk_share: current.pk_share,
                sk_poly_sum: current.sk_poly_sum,
                ciphertext_output: ciphertext_output.clone(),
                signed_pk_generation_proof: current.signed_pk_generation_proof,
                signed_sk_share_computation_proof: current.signed_sk_share_computation_proof,
                signed_sk_share_encryption_proofs: current.signed_sk_share_encryption_proofs,
                outgoing_prf_keys: current.outgoing_prf_keys,
                incoming_prf_keys: current.incoming_prf_keys,
                decryptors: decryptors.clone(),
            });

            s.new_state(next)
        })?;

        self.issue_decryption_share_request(ec)
    }

    /// Issue one share calculation per process. The worker retries the same request, and the
    /// actor redelivers it when its result does not arrive.
    pub(in crate::actors::threshold_keyshare) fn issue_decryption_share_request(
        &mut self,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        if self.pending.decryption_share_request.is_some() {
            return Ok(());
        }
        if self.publish_decryption_share_request(ec.clone())? {
            self.pending.decryption_share_request = Some(IssuedDecryptionWork {
                ec,
                last_sent: std::time::Instant::now(),
                redeliveries: 0,
            });
        }
        Ok(())
    }

    /// Publish the share calculation for the saved ciphertext under a new correlation ID. Returns
    /// false when the chain public-key context is not recovered yet.
    fn publish_decryption_share_request(&mut self, ec: EventContext<Sequenced>) -> Result<bool> {
        let state = self.state.try_get()?;
        if !self.public_key_context_is_recovered(&state) {
            return Ok(false);
        }
        let e3_id = state.get_e3_id();
        let decrypting: Decrypting = state.clone().try_into()?;
        // Standbys retain the ciphertext for a later designated set, but must not dispatch
        // an operation that the FHE API correctly rejects for a non-member.
        if !decrypting
            .decryptors
            .contains(&(party_index(state.party_id)? + 1))
        {
            return Ok(false);
        }
        let trbfv_config = state.get_trbfv_config();
        let event = ComputeRequest::trbfv(
            TrBFVRequest::CalculateDecryptionShare(CalculateDecryptionShareRequest {
                name: format!("party_id({})", state.party_id),
                ciphertexts: decrypting.ciphertext_output,
                sk_poly_sum: decrypting.sk_poly_sum,
                trbfv_config,
                party_idx: party_index(state.party_id)?,
                decryptors: decrypting.decryptors,
                outgoing_prf_keys: decrypting.outgoing_prf_keys,
                incoming_prf_keys: decrypting.incoming_prf_keys,
            }),
            CorrelationId::new(),
            e3_id.clone(),
        );
        self.bus.publish(event, ec)?;
        Ok(true)
    }

    pub(in crate::actors::threshold_keyshare) fn issue_decryption_proof_request(
        &mut self,
        mut pending: ShareDecryptionProofPending,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        if self.pending.decryption_proof_request.is_some() {
            return Ok(());
        }
        let state = self.state.try_get()?;
        if self
            .canonical_keys
            .repair_request(&state.e3_id, &mut pending.proof_request)
            .is_err()
        {
            return Ok(());
        }
        self.recovery.try_mutate(&ec, |mut recovery| {
            recovery.share_decryption_proof_pending =
                Some(TypedEvent::new(pending.clone(), ec.clone()));
            recovery.last_ec = Some(ec.clone());
            Ok(recovery)
        })?;
        self.bus.publish(pending, ec.clone())?;
        self.pending.decryption_proof_request = Some(IssuedDecryptionWork {
            ec,
            last_sent: std::time::Instant::now(),
            redeliveries: 0,
        });
        Ok(())
    }

    /// Send outstanding decryption work again when its result has not arrived for
    /// `DECRYPTION_REDELIVERY_DELAY`, at most `MAX_DECRYPTION_REDELIVERIES` times per phase. A lost
    /// request or result otherwise holds the node in its phase until a restart.
    pub(in crate::actors::threshold_keyshare) fn redeliver_decryption_work(
        &mut self,
        now: std::time::Instant,
    ) -> Result<()> {
        if !self.effects_enabled {
            return Ok(());
        }
        let due = |work: &IssuedDecryptionWork| {
            work.redeliveries < MAX_DECRYPTION_REDELIVERIES
                && now.saturating_duration_since(work.last_sent) >= DECRYPTION_REDELIVERY_DELAY
        };
        let state = self.state.try_get()?;
        match state.state {
            KeyshareState::Decrypting(_) => {
                let Some(work) = self.pending.decryption_share_request.clone().filter(due) else {
                    return Ok(());
                };
                if self.publish_decryption_share_request(work.ec.clone())? {
                    info!(
                        e3_id = %state.e3_id,
                        redelivery = work.redeliveries + 1,
                        "Redelivering a decryption-share request whose result has not arrived"
                    );
                    self.pending.decryption_share_request = Some(IssuedDecryptionWork {
                        last_sent: now,
                        redeliveries: work.redeliveries + 1,
                        ..work
                    });
                }
            }
            KeyshareState::GeneratingDecryptionProof(_) => {
                let Some(work) = self.pending.decryption_proof_request.clone().filter(due) else {
                    return Ok(());
                };
                let Some(pending) = self.recovery.try_get()?.share_decryption_proof_pending else {
                    return Ok(());
                };
                let mut pending = pending.into_inner();
                // A fresh value per redelivery, also after a restart: a counter would repeat the
                // values of replayed earlier requests.
                pending.redelivery = rand::random::<u64>().max(1);
                info!(
                    e3_id = %state.e3_id,
                    redelivery = pending.redelivery,
                    "Redelivering a C6 proof request whose result has not arrived"
                );
                self.bus.publish(pending, work.ec.clone())?;
                self.pending.decryption_proof_request = Some(IssuedDecryptionWork {
                    last_sent: now,
                    redeliveries: work.redeliveries + 1,
                    ..work
                });
            }
            _ => {}
        }
        Ok(())
    }

    /// CalculateDecryptionShareResponse — publish ShareDecryptionProofPending
    /// so ProofRequestActor generates and signs C6 proofs.
    pub fn handle_calculate_decryption_share_response(
        &mut self,
        res: TypedEvent<ComputeResponse>,
    ) -> Result<()> {
        let (res, ec) = res.into_components();
        let msg: CalculateDecryptionShareResponse = res.try_into()?;
        let state = self.state.try_get()?;
        let e3_id = state.e3_id.clone();
        if !matches!(state.state, KeyshareState::Decrypting(_)) {
            tracing::debug!(
                e3_id = %e3_id,
                state = state.variant_name(),
                "Ignoring a decryption-share response after leaving Decrypting"
            );
            return Ok(());
        }
        let decrypting: Decrypting = state.clone().try_into()?;
        let expected_digest = e3_trbfv::calculate_threshold_decryption::decryption_context_digest(
            &decrypting.decryptors,
            &decrypting.ciphertext_output,
        );
        if msg.context_digest != expected_digest {
            tracing::debug!(
                e3_id = %e3_id,
                "Ignoring a decryption-share response for a different decryptor set"
            );
            return Ok(());
        }
        let d_share_poly = msg.d_share_poly;

        anyhow::ensure!(
            self.public_key_context_is_recovered(&state),
            "chain public-key context is unavailable for C6 proof"
        );
        let aggregated_pk_bytes = state
            .aggregated_pk
            .clone()
            .ok_or_else(|| anyhow!("Aggregated public key not available for C6 proof"))?;
        let decryption_domain = state
            .decryption_domain
            .ok_or_else(|| anyhow!("E3 decryption domain not available for C6 proof"))?;

        let threshold_preset = self
            .share_enc_preset
            .threshold_counterpart()
            .ok_or_else(|| {
                anyhow!(
                    "No threshold counterpart for preset {:?}",
                    self.share_enc_preset
                )
            })?;

        info!("Publishing ShareDecryptionProofPending for C6 proof generation...");

        let committee_size = state.committee_size()?;

        // Publish pending event before transitioning state so a publish
        // failure leaves us in Decrypting (retryable) rather than
        // GeneratingDecryptionProof (no retry path).
        let event = ShareDecryptionProofPending {
            e3_id: e3_id.clone(),
            party_id: state.party_id,
            node: state.address.clone(),
            decryption_share: d_share_poly.clone(),
            proof_request: ThresholdShareDecryptionProofRequest {
                ciphertext_bytes: decrypting.ciphertext_output.clone(),
                aggregated_pk_bytes,
                sk_poly_sum: decrypting.sk_poly_sum.clone(),
                e_fresh: msg.e_fresh,
                e_signed: msg.e_signed,
                d_share_bytes: d_share_poly.clone(),
                decryption_domain,
                params_preset: threshold_preset,
                committee_size,
                party_idx: party_index(state.party_id)?,
                decryptors: decrypting.decryptors.clone(),
                outgoing_prf_keys: decrypting.outgoing_prf_keys.clone(),
                incoming_prf_keys: decrypting.incoming_prf_keys.clone(),
            },
            redelivery: 0,
            decryptors: decrypting.decryptors.clone(),
        };
        self.issue_decryption_proof_request(event, ec.clone())?;

        // Transition to GeneratingDecryptionProof state
        self.state.try_mutate(&ec, |s| {
            use KeyshareState as K;
            s.new_state(K::GeneratingDecryptionProof(GeneratingDecryptionProof {
                pk_share: decrypting.pk_share.clone(),
                decryption_share: d_share_poly,
                signed_pk_generation_proof: decrypting.signed_pk_generation_proof.clone(),
                signed_sk_share_computation_proof: decrypting
                    .signed_sk_share_computation_proof
                    .clone(),
                signed_sk_share_encryption_proofs: decrypting
                    .signed_sk_share_encryption_proofs
                    .clone(),
                ciphertext_output: decrypting.ciphertext_output.clone(),
                sk_poly_sum: decrypting.sk_poly_sum.clone(),
                outgoing_prf_keys: decrypting.outgoing_prf_keys.clone(),
                incoming_prf_keys: decrypting.incoming_prf_keys.clone(),
                decryptors: decrypting.decryptors.clone(),
            }))
        })?;
        Ok(())
    }

    /// Recompute a share for a new decryptor set. Old shares stay bound to the previous set.
    pub fn handle_fresh_decryption_round(
        &mut self,
        msg: TypedEvent<FreshDecryptionRound>,
    ) -> Result<()> {
        let (data, ec) = msg.into_components();
        if ec.source() == e3_events::EventSource::Net {
            return Ok(());
        }
        self.apply_fresh_decryption_round(TypedEvent::new(data, ec))
    }

    pub(super) fn apply_fresh_decryption_round(
        &mut self,
        msg: TypedEvent<FreshDecryptionRound>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();
        let state = self.state.try_get()?;
        if msg.e3_id != state.e3_id {
            return Ok(());
        }
        // This is a local derived control event, not an aggregator's authority to select a set.
        // Every node independently derives the same set after authenticated exclusions.
        let canonical = canonical_decryptors(
            state.honest_parties.as_ref(),
            &state.expelled_parties,
            reconstruction_count(state.threshold_m)?,
        )?;
        if msg.decryptors != canonical {
            return Ok(());
        }
        let previous = match &state.state {
            KeyshareState::Decrypting(round) => Some(&round.decryptors),
            KeyshareState::GeneratingDecryptionProof(round) => Some(&round.decryptors),
            KeyshareState::Completed(round) => Some(&round.decryptors),
            _ => None,
        };
        if previous == Some(&msg.decryptors) {
            return if self.effects_enabled && matches!(state.state, KeyshareState::Decrypting(_)) {
                self.issue_decryption_share_request(ec)
            } else {
                Ok(())
            };
        }
        let party = party_index(state.party_id)?
            .checked_add(1)
            .context("party id cannot become a 1-based decryptor id")?;
        let included = msg.decryptors.contains(&party);
        match state.state {
            KeyshareState::Decrypting(_) => {
                self.state.try_mutate(&ec, |s| {
                    let KeyshareState::Decrypting(mut current) = s.state.clone() else {
                        bail!("expected Decrypting");
                    };
                    current.decryptors = msg.decryptors.clone();
                    s.new_state(KeyshareState::Decrypting(current))
                })?;
                self.begin_fresh_decryption_round(&ec)?;
                if included && self.effects_enabled {
                    self.issue_decryption_share_request(ec)
                } else {
                    Ok(())
                }
            }
            KeyshareState::GeneratingDecryptionProof(_) => {
                self.state.try_mutate(&ec, |s| {
                    let KeyshareState::GeneratingDecryptionProof(current) = s.state.clone() else {
                        bail!("expected GeneratingDecryptionProof");
                    };
                    s.new_state(KeyshareState::Decrypting(decrypting_for_round(
                        current.pk_share,
                        current.sk_poly_sum,
                        current.ciphertext_output,
                        current.signed_pk_generation_proof,
                        current.signed_sk_share_computation_proof,
                        current.signed_sk_share_encryption_proofs,
                        current.outgoing_prf_keys,
                        current.incoming_prf_keys,
                        msg.decryptors.clone(),
                    )))
                })?;
                self.begin_fresh_decryption_round(&ec)?;
                if included && self.effects_enabled {
                    self.issue_decryption_share_request(ec)
                } else {
                    Ok(())
                }
            }
            KeyshareState::Completed(_) => {
                self.state.try_mutate(&ec, |s| {
                    let KeyshareState::Completed(current) = s.state.clone() else {
                        bail!("expected Completed");
                    };
                    s.new_state(KeyshareState::Decrypting(decrypting_for_round(
                        current.pk_share,
                        current.sk_poly_sum,
                        current.ciphertext_output,
                        current.signed_pk_generation_proof,
                        current.signed_sk_share_computation_proof,
                        current.signed_sk_share_encryption_proofs,
                        current.outgoing_prf_keys,
                        current.incoming_prf_keys,
                        msg.decryptors.clone(),
                    )))
                })?;
                self.begin_fresh_decryption_round(&ec)?;
                if included && self.effects_enabled {
                    self.issue_decryption_share_request(ec)
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    /// Retire the issued-work slots of the round that a fresh decryptor set replaces. Each round
    /// issues its own share and proof requests and owns its redelivery budget.
    fn begin_fresh_decryption_round(&mut self, ec: &EventContext<Sequenced>) -> Result<()> {
        self.pending.decryption_share_request = None;
        self.pending.decryption_proof_request = None;
        self.recovery.try_mutate(ec, |mut recovery| {
            recovery.share_decryption_proof_pending = None;
            recovery.last_ec = Some(ec.clone());
            Ok(recovery)
        })
    }

    pub fn handle_decryption_share_proof_signed(
        &mut self,
        msg: TypedEvent<DecryptionShareProofSigned>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();
        let state = self.state.try_get()?;
        if msg.e3_id != state.e3_id {
            return Ok(());
        }
        let KeyshareState::GeneratingDecryptionProof(current) = &state.state else {
            return Ok(());
        };
        if current.decryptors != msg.decryptors {
            return Ok(());
        }

        self.state.try_mutate(&ec, |s| {
            let KeyshareState::GeneratingDecryptionProof(current) = s.state.clone() else {
                bail!("expected GeneratingDecryptionProof");
            };
            if current.decryptors != msg.decryptors {
                bail!("the signed proof belongs to a different decryptor set");
            }
            info!("Decryption share sending process is complete");
            s.new_state(KeyshareState::Completed(Completed {
                pk_share: current.pk_share,
                ciphertext_output: current.ciphertext_output,
                sk_poly_sum: current.sk_poly_sum,
                signed_pk_generation_proof: current.signed_pk_generation_proof,
                signed_sk_share_computation_proof: current.signed_sk_share_computation_proof,
                signed_sk_share_encryption_proofs: current.signed_sk_share_encryption_proofs,
                outgoing_prf_keys: current.outgoing_prf_keys,
                incoming_prf_keys: current.incoming_prf_keys,
                decryptors: current.decryptors,
            }))
        })?;

        Ok(())
    }
}

fn decrypting_for_round(
    pk_share: ArcBytes,
    sk_poly_sum: SensitiveBytes,
    ciphertext_output: Vec<ArcBytes>,
    signed_pk_generation_proof: Option<SignedProofPayload>,
    signed_sk_share_computation_proof: Option<SignedProofPayload>,
    signed_sk_share_encryption_proofs: Vec<SignedProofPayload>,
    outgoing_prf_keys: Vec<SensitiveBytes>,
    incoming_prf_keys: Vec<SensitiveBytes>,
    decryptors: Vec<u32>,
) -> Decrypting {
    Decrypting {
        pk_share,
        sk_poly_sum,
        ciphertext_output,
        signed_pk_generation_proof,
        signed_sk_share_computation_proof,
        signed_sk_share_encryption_proofs,
        outgoing_prf_keys,
        incoming_prf_keys,
        decryptors,
    }
}

/// The number of shares that reconstruct a synchronized decryption: one more than the threshold.
pub(super) fn reconstruction_count(threshold_m: u64) -> Result<usize> {
    usize::try_from(threshold_m)
        .map(|threshold| threshold + 1)
        .map_err(|_| anyhow!("threshold does not fit usize"))
}

fn party_index(party_id: u64) -> Result<u32> {
    u32::try_from(party_id).map_err(|_| anyhow!("party id does not fit a 32-bit decryptor index"))
}

/// The lowest honest parties as strictly increasing 1-based decryptor ids.
pub(super) fn canonical_decryptors(
    honest: Option<&std::collections::BTreeSet<u64>>,
    excluded: &std::collections::HashSet<u64>,
    reconstruction: usize,
) -> Result<Vec<u32>> {
    let Some(honest) = honest else {
        bail!("honest party set is missing");
    };
    let eligible: Vec<_> = honest.iter().filter(|id| !excluded.contains(id)).collect();
    if eligible.len() < reconstruction {
        bail!(
            "honest party set has {} members and needs {reconstruction} decryptors",
            eligible.len()
        );
    }
    eligible
        .into_iter()
        .take(reconstruction)
        .map(|id| {
            let id = u32::try_from(*id).map_err(|_| anyhow!("honest party id does not fit u32"))?;
            id.checked_add(1)
                .context("honest party id cannot become a 1-based decryptor id")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::canonical_decryptors;
    use std::collections::{BTreeSet, HashSet};

    #[test]
    fn missing_honest_set_is_rejected() {
        let error = canonical_decryptors(None, &HashSet::new(), 2).unwrap_err();
        assert!(error.to_string().contains("missing"));
    }

    #[test]
    fn short_honest_set_is_rejected() {
        let honest: BTreeSet<u64> = [0, 1].into_iter().collect();
        let error = canonical_decryptors(Some(&honest), &HashSet::new(), 3).unwrap_err();
        assert!(error.to_string().contains("needs 3"));
    }

    #[test]
    fn lowest_honest_ids_become_one_based_decryptors() {
        let honest: BTreeSet<u64> = [4, 1, 0, 7].into_iter().collect();
        let ids = canonical_decryptors(Some(&honest), &HashSet::from([7]), 3).unwrap();
        assert_eq!(ids, vec![1, 2, 5]);
    }
}
