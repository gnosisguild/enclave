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

        // If we are already in Decrypting (or beyond), this is a duplicate
        // event — e.g. replayed from the EventStore during crash recovery.
        // Re-issue the compute request idempotently (downstream aggregation
        // deduplicates by party_id) and skip the state transition.
        {
            let state = self.state.try_get()?;
            match &state.state {
                KeyshareState::Decrypting(_) => {
                    info!(
                        e3_id = %state.e3_id,
                        "CiphertextOutputPublished received while already Decrypting — re-issuing decryption-share request"
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
        let (party_idx, decryptors) = {
            let state = self.state.try_get()?;
            (
                party_index(state.party_id)?,
                canonical_decryptors(
                    state.honest_parties.as_ref(),
                    reconstruction_count(state.threshold_m)?,
                )?,
            )
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

        let state = self.state.try_get()?;
        let e3_id = state.get_e3_id();
        let decrypting: Decrypting = state.clone().try_into()?;
        let trbfv_config = state.get_trbfv_config();
        let event = ComputeRequest::trbfv(
            TrBFVRequest::CalculateDecryptionShare(CalculateDecryptionShareRequest {
                name: format!("party_id({})", state.party_id),
                ciphertexts: ciphertext_output,
                sk_poly_sum: decrypting.sk_poly_sum,
                trbfv_config,
                party_idx,
                decryptors,
                outgoing_prf_keys: decrypting.outgoing_prf_keys,
                incoming_prf_keys: decrypting.incoming_prf_keys,
            }),
            CorrelationId::new(),
            e3_id.clone(),
        );
        self.bus.publish(event, ec)?; // CalculateDecryptionShareRequest
        Ok(())
    }

    /// (Re)issue the `CalculateDecryptionShare` compute request from the current
    /// `Decrypting` state. Factored out of `handle_ciphertext_output_published` so the
    /// boot-time resume path can re-drive the decryption-share computation idempotently
    /// (the resulting `DecryptionshareCreated` is deduped by `party_id` at the aggregator).
    pub(in crate::actors::threshold_keyshare) fn issue_decryption_share_request(
        &self,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        let state = self.state.try_get()?;
        let e3_id = state.get_e3_id();
        let decrypting: Decrypting = state.clone().try_into()?;
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
                state = %state.variant_name(),
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
        };
        self.recovery.try_mutate(&ec, |mut recovery| {
            recovery.share_decryption_proof_pending =
                Some(TypedEvent::new(event.clone(), ec.clone()));
            recovery.last_ec = Some(ec.clone());
            Ok(recovery)
        })?;
        self.bus.publish(event, ec.clone())?;

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
        let (msg, ec) = msg.into_components();
        let state = self.state.try_get()?;
        if msg.e3_id != state.e3_id {
            return Ok(());
        }
        let party = party_index(state.party_id)?
            .checked_add(1)
            .context("party id cannot become a 1-based decryptor id")?;
        let included = msg.decryptors.contains(&party);
        if !included {
            if matches!(state.state, KeyshareState::Decrypting(_)) {
                self.state.try_mutate(&ec, |s| {
                    let KeyshareState::Decrypting(mut current) = s.state.clone() else {
                        bail!("expected Decrypting");
                    };
                    current.decryptors = msg.decryptors.clone();
                    s.new_state(KeyshareState::Decrypting(current))
                })?;
            }
            return Ok(());
        }
        match state.state {
            KeyshareState::Decrypting(_) => {
                self.state.try_mutate(&ec, |s| {
                    let KeyshareState::Decrypting(mut current) = s.state.clone() else {
                        bail!("expected Decrypting");
                    };
                    current.decryptors = msg.decryptors.clone();
                    s.new_state(KeyshareState::Decrypting(current))
                })?;
                self.issue_decryption_share_request(ec)
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
                self.issue_decryption_share_request(ec)
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
                self.issue_decryption_share_request(ec)
            }
            _ => Ok(()),
        }
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
    pk_share: e3_utils::utility_types::ArcBytes,
    sk_poly_sum: e3_crypto::SensitiveBytes,
    ciphertext_output: Vec<e3_utils::utility_types::ArcBytes>,
    signed_pk_generation_proof: Option<e3_events::SignedProofPayload>,
    signed_sk_share_computation_proof: Option<e3_events::SignedProofPayload>,
    signed_sk_share_encryption_proofs: Vec<e3_events::SignedProofPayload>,
    outgoing_prf_keys: Vec<e3_crypto::SensitiveBytes>,
    incoming_prf_keys: Vec<e3_crypto::SensitiveBytes>,
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

fn reconstruction_count(threshold_m: u64) -> Result<usize> {
    usize::try_from(threshold_m)
        .map(|threshold| threshold + 1)
        .map_err(|_| anyhow!("threshold does not fit usize"))
}

fn party_index(party_id: u64) -> Result<u32> {
    u32::try_from(party_id).map_err(|_| anyhow!("party id does not fit a 32-bit decryptor index"))
}

fn canonical_decryptors(
    honest: Option<&std::collections::BTreeSet<u64>>,
    reconstruction: usize,
) -> Result<Vec<u32>> {
    let Some(honest) = honest else {
        bail!("honest party set is missing");
    };
    if honest.len() < reconstruction {
        bail!(
            "honest party set has {} members and needs {reconstruction} decryptors",
            honest.len()
        );
    }
    honest
        .iter()
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
    use std::collections::BTreeSet;

    #[test]
    fn missing_honest_set_is_rejected() {
        let error = canonical_decryptors(None, 2).unwrap_err();
        assert!(error.to_string().contains("missing"));
    }

    #[test]
    fn short_honest_set_is_rejected() {
        let honest: BTreeSet<u64> = [0, 1].into_iter().collect();
        let error = canonical_decryptors(Some(&honest), 3).unwrap_err();
        assert!(error.to_string().contains("needs 3"));
    }

    #[test]
    fn lowest_honest_ids_become_one_based_decryptors() {
        let honest: BTreeSet<u64> = [4, 1, 0, 7].into_iter().collect();
        let ids = canonical_decryptors(Some(&honest), 3).unwrap();
        assert_eq!(ids, vec![1, 2, 5]);
    }
}
