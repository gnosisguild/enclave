// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use super::{admission::*, *};
use e3_events::{ComputeRequestKind, PartyVerificationResult, VerifyShareProofsResponse};

/// Supply a controlled verifier outcome at the workflow boundary. This tests signed voting and
/// asymmetric delivery, not cryptographic verification, policy reads, or decryption convergence.
fn reverify_failure(
    voting: &mut AccusationVoting,
    accusation: ProofFailureAccusation,
    failure: &ProofVerificationFailed,
) -> Vec<VoteAction> {
    let actions = voting.on_accusation_received(accusation, &ctx());
    assert!(!actions.iter().any(|action| matches!(
        action,
        VoteAction::PublishVote { .. } | VoteAction::PublishQuorum { .. }
    )));
    let request = actions
        .into_iter()
        .find_map(|action| match action {
            VoteAction::DispatchZk { request, .. } => Some(request),
            _ => None,
        })
        .expect("a voter with no local C6 result must request independent verification");
    let ComputeRequestKind::Zk(ZkRequest::ReverifyAccusedProof(batch)) = &request.request else {
        panic!("expected forwarded-proof verification");
    };
    assert_eq!(batch.committee_size, CiphernodesCommitteeSize::Small);
    assert_eq!(batch.party_proofs.len(), 1);
    assert_eq!(
        batch.party_proofs[0].sender_party_id,
        failure.accused_party_id
    );
    assert_eq!(
        batch.party_proofs[0].signed_proofs,
        vec![failure.signed_payload.clone()]
    );
    let response = ComputeResponse::zk(
        ZkResponse::VerifyShareProofs(VerifyShareProofsResponse {
            party_results: vec![PartyVerificationResult {
                sender_party_id: failure.accused_party_id,
                all_verified: false,
                failed_signed_payload: Some(failure.signed_payload.clone()),
                recovered_address: Some(failure.accused_address),
            }],
        }),
        request.correlation_id,
        voting.e3_id.clone(),
    );
    voting.handle_reverification_response(TypedEvent::new(response, ctx()))
}

#[test]
fn c6_delayed_voter_converges_only_while_the_accusation_window_is_live() {
    let size = CiphernodesCommitteeSize::Small.values();
    let signers: Vec<_> = (1..=size.n as u8).map(signer).collect();
    let committee: Vec<_> = signers.iter().map(|signer| signer.address()).collect();
    let accused_party = size.n - 1;
    let accused = &signers[accused_party];
    let delayed_party = 5;
    let failure = proof_failure(
        accused,
        accused_party as u64,
        ProofType::C6ThresholdShareDecryption,
    );
    // H voters can complete without this other committee member. No quorum or exclusion is
    // injected: each vote is produced by the real workflow after its local verifier result.
    let mut online: Vec<_> = signers
        .iter()
        .enumerate()
        .filter(|(party, _)| *party != accused_party && *party != delayed_party)
        .take(size.h)
        .map(|(_, signer)| voting_with(signer, committee.clone(), size.threshold, size.h))
        .collect();
    let actions = online[0].on_local_proof_failure(failure.clone(), &ctx());
    let accusation = actions
        .iter()
        .find_map(|action| match action {
            VoteAction::PublishAccusation { accusation, .. } => Some(accusation.clone()),
            _ => None,
        })
        .unwrap();
    let mut votes = Vec::new();
    for action in actions {
        if let VoteAction::PublishVote { vote, .. } = action {
            votes.push(vote);
        }
    }
    for voter in online.iter_mut().skip(1) {
        for action in reverify_failure(voter, accusation.clone(), &failure) {
            if let VoteAction::PublishVote { vote, .. } = action {
                votes.push(vote);
            }
        }
    }
    assert_eq!(votes.len(), size.h);
    let voters: Vec<_> = votes.iter().map(|vote| vote.voter).collect();
    for voter in &mut online {
        let actions: Vec<_> = votes
            .iter()
            .flat_map(|vote| voter.on_vote_received(vote.clone(), &ctx()))
            .collect();
        assert_quorum(&actions, accused.address(), &voters);
    }

    for now in [NOW + 1, accusation.deadline, accusation.deadline + 1] {
        let mut delayed = voting_with(
            &signers[delayed_party],
            committee.clone(),
            size.threshold,
            size.h,
        );
        delayed.clock = Arc::new(FixedClock(now));
        assert!(votes.iter().all(|vote| delayed.verify_vote_signature(vote)));
        if now < accusation.deadline {
            let mut actions = reverify_failure(&mut delayed, accusation.clone(), &failure);
            assert!(actions.iter().any(|action| matches!(action,
                VoteAction::PublishVote { vote, .. } if vote.voter == signers[delayed_party].address())));
            for vote in &votes {
                actions.extend(delayed.on_vote_received(vote.clone(), &ctx()));
            }
            let expected: Vec<_> = std::iter::once(signers[delayed_party].address())
                .chain(voters.iter().copied().take(size.h - 1))
                .collect();
            assert_quorum(&actions, accused.address(), &expected);
        } else {
            // The very same authenticated messages no longer start voting. Preserving this
            // admission rule exposes the missing completed-quorum recovery path; it is not an
            // end-to-end liveness success or permission to reinterpret expired live messages.
            assert!(delayed
                .on_accusation_received(accusation.clone(), &ctx())
                .is_empty());
            for vote in &votes {
                assert!(delayed.on_vote_received(vote.clone(), &ctx()).is_empty());
            }
            assert!(delayed
                .on_accusation_received(accusation.clone(), &ctx())
                .is_empty());
            assert!(delayed.pending.is_empty());
            assert!(delayed.pending_reverifications.is_empty());
        }
    }
}
