// SPDX-License-Identifier: LGPL-3.0-only

use super::*;

async fn small_aggregator() -> Result<(
    ThresholdPlaintextAggregator,
    Addr<HistoryCollector<InterfoldEvent>>,
    E3id,
)> {
    let state = ThresholdPlaintextAggregatorState::init(
        9,
        19,
        Seed([0; 32]),
        test_ciphertexts()[..1].to_vec(),
        test_params(),
    );
    let (mut aggregator, history, e3_id) = build_plaintext_aggregator(state, true).await?;
    aggregator.committee_size = CiphernodesCommitteeSize::Small;
    aggregator.committee_addresses = (0..19).map(|party| test_signer(party).address()).collect();
    aggregator.honest_committee_addresses = aggregator.committee_addresses[5..].to_vec();
    Ok((aggregator, history, e3_id))
}

fn collect(
    aggregator: &mut ThresholdPlaintextAggregator,
    parties: impl Iterator<Item = u64>,
) -> Result<()> {
    let e3_id = aggregator.e3_id.clone();
    let state = aggregator.state.try_get()?;
    let (threshold, rejected) = match &state {
        ThresholdPlaintextAggregatorState::Collecting(s) => {
            (s.threshold_m, s.rejected_parties.clone())
        }
        ThresholdPlaintextAggregatorState::VerifyingC6(s) => {
            (s.threshold_m, s.rejected_parties.clone())
        }
        _ => return Ok(()),
    };
    let ids = ThresholdPlaintextAggregation::fresh_decryptor_ids(
        &aggregator.committee_addresses,
        &aggregator.honest_committee_addresses,
        &rejected,
        threshold,
    )?;
    collect_for(&e3_id, aggregator, parties, &ids)
}

fn collect_for(
    e3_id: &E3id,
    aggregator: &mut ThresholdPlaintextAggregator,
    parties: impl Iterator<Item = u64>,
    decryptors: &[u32],
) -> Result<()> {
    let ec = test_ctx(EffectsEnabled::new());
    for party in parties {
        let (share, proofs) = share_with_matching_commitment_in_set(
            e3_id,
            party,
            &test_ciphertexts()[..1],
            decryptors,
        );
        aggregator.add_share(party, share, proofs, decryptors, &ec)?;
    }
    Ok(())
}

#[actix::test]
async fn small_decrypts_with_ten_verified_shares_not_all_fourteen() -> Result<()> {
    for missing in 0..=5 {
        let (mut aggregator, history, _) = small_aggregator().await?;
        collect(&mut aggregator, 5..19 - missing)?;
        if missing == 5 {
            assert!(matches!(
                aggregator.state.get(),
                Some(ThresholdPlaintextAggregatorState::Collecting(_))
            ));
            continue;
        }
        let outcome = c6_completion(&aggregator, BTreeSet::new());
        aggregator.handle_c6_verification_complete(outcome)?;
        let event = next_event(&history).await?;
        assert!(matches!(
            event.get_data(),
            InterfoldEventData::ComputeRequest(_)
        ));
        let state = Computing::try_from(aggregator.state.try_get()?)?;
        assert_eq!(state.shares.len(), 10);
        assert_eq!(state.shares.first().unwrap().0, 5);
        assert_eq!(aggregator.recovery.try_get()?.honest_c6_proofs.len(), 10);
    }
    Ok(())
}

#[actix::test]
async fn rejected_share_collects_a_fresh_decryptor_set() -> Result<()> {
    for corrupt_saved_share in [false, true] {
        let (mut aggregator, history, e3_id) = small_aggregator().await?;
        collect(&mut aggregator, 5..15)?;
        // A share that arrives before the retry belongs to the old decryptor set.
        collect(&mut aggregator, 15..16)?;
        if corrupt_saved_share {
            aggregator
                .state
                .try_mutate(&test_ctx(EffectsEnabled::new()), |mut state| {
                    if let ThresholdPlaintextAggregatorState::VerifyingC6(batch) = &mut state {
                        batch.shares.insert(5, vec![ArcBytes::from_bytes(&[0])]);
                    }
                    Ok(state)
                })?;
        }
        let dishonest_parties = if corrupt_saved_share {
            BTreeSet::new()
        } else {
            BTreeSet::from([5])
        };
        let old_result = c6_completion(&aggregator, dishonest_parties);
        aggregator.handle_c6_verification_complete(old_result.clone())?;
        actix::clock::sleep(std::time::Duration::from_millis(20)).await;
        let collecting = Collecting::try_from(aggregator.state.try_get()?)?;
        assert!(collecting.shares.is_empty());
        assert!(collecting.rejected_parties.contains(&5));
        let decryptors = collecting.expected_decryptors.clone();
        assert_eq!(decryptors, (7u32..=16).collect::<Vec<_>>());

        collect_for(&e3_id, &mut aggregator, 5..6, &decryptors)?;
        collect_for(&e3_id, &mut aggregator, 6..7, &[])?;
        assert!(Collecting::try_from(aggregator.state.try_get()?)?
            .shares
            .is_empty());
        collect_for(&e3_id, &mut aggregator, 6..16, &decryptors)?;
        actix::clock::sleep(std::time::Duration::from_millis(20)).await;
        let batch = VerifyingC6::try_from(aggregator.state.try_get()?)?;
        assert_eq!(
            batch.shares.keys().copied().collect::<Vec<_>>(),
            (6..16).collect::<Vec<_>>()
        );
        // A result for the previous batch must not authorize the new shares.
        aggregator.handle_c6_verification_complete(old_result)?;
        assert!(matches!(
            aggregator.state.get(),
            Some(ThresholdPlaintextAggregatorState::VerifyingC6(_))
        ));
        let replacement_result = c6_completion(&aggregator, BTreeSet::new());
        aggregator.handle_c6_verification_complete(replacement_result)?;
        assert!(matches!(
            aggregator.state.get(),
            Some(ThresholdPlaintextAggregatorState::Computing(_))
        ));
        actix::clock::sleep(std::time::Duration::from_millis(10)).await;
        let events = history.send(GetEvents::<InterfoldEvent>::new()).await?;
        assert!(events.iter().any(|event| matches!(
            event.get_data(),
            InterfoldEventData::FreshDecryptionRound(_)
        )));
        assert!(!events
            .iter()
            .any(|event| matches!(event.get_data(), InterfoldEventData::E3Failed(_))));
    }
    Ok(())
}

#[actix::test]
async fn replayed_results_survive_restart_and_wait_for_effects_and_promotion() -> Result<()> {
    let (mut aggregator, history, _) = small_aggregator().await?;
    collect(&mut aggregator, 5..16)?;
    aggregator.effects_enabled = false;
    aggregator.is_aggregator = false;
    let rejected = c6_completion(&aggregator, BTreeSet::from([5]));
    aggregator.handle_c6_verification_complete(rejected)?;
    // Snapshot both stores, as restart hydration does, and discard process-local work.
    let state = bincode::deserialize(&bincode::serialize(&aggregator.state.try_get()?)?)?;
    let recovery = bincode::deserialize(&bincode::serialize(&aggregator.recovery.try_get()?)?)?;
    aggregator.state = test_persistable(state);
    aggregator.recovery = test_persistable(recovery);
    aggregator.pending = PendingDecryptionWork::default();
    aggregator.resume_in_flight_work(test_ctx(EffectsEnabled::new()))?;
    assert!(history
        .send(GetEvents::<InterfoldEvent>::new())
        .await?
        .is_empty());
    aggregator.effects_enabled = true;
    aggregator.resume_in_flight_work(test_ctx(EffectsEnabled::new()))?;
    assert!(history
        .send(GetEvents::<InterfoldEvent>::new())
        .await?
        .is_empty());
    aggregator.is_aggregator = true;
    aggregator.resume_in_flight_work(test_ctx(EffectsEnabled::new()))?;
    deliver_resume(&mut aggregator, &history).await?;
    let collecting = Collecting::try_from(aggregator.state.try_get()?)?;
    assert!(collecting.shares.is_empty());
    assert!(collecting.rejected_parties.contains(&5));
    let decryptors = collecting.expected_decryptors.clone();
    assert_eq!(decryptors, (7u32..=16).collect::<Vec<_>>());
    let e3_id = aggregator.e3_id.clone();
    collect_for(&e3_id, &mut aggregator, 6..16, &decryptors)?;
    let batch = VerifyingC6::try_from(aggregator.state.try_get()?)?;
    assert_eq!(
        batch.shares.keys().copied().collect::<Vec<_>>(),
        (6..16).collect::<Vec<_>>()
    );
    let completion = c6_completion(&aggregator, BTreeSet::new());
    aggregator.effects_enabled = false;
    aggregator.handle_c6_verification_complete(completion)?;
    aggregator.effects_enabled = true;
    aggregator.resume_in_flight_work(test_ctx(EffectsEnabled::new()))?;
    deliver_resume(&mut aggregator, &history).await?;
    assert!(matches!(
        aggregator.state.get(),
        Some(ThresholdPlaintextAggregatorState::Computing(_))
    ));
    Ok(())
}

#[actix::test]
async fn a_network_verdict_cannot_authorize_decryption() -> Result<()> {
    let (mut aggregator, history, _) = small_aggregator().await?;
    collect(&mut aggregator, 5..15)?;
    let (payload, ec) = c6_completion(&aggregator, BTreeSet::new()).into_components();
    aggregator.handle_c6_verification_complete(TypedEvent::new(
        payload,
        ec.with_source(e3_events::EventSource::Net),
    ))?;
    assert!(aggregator.recovery.try_get()?.c6_outcomes.is_empty());
    assert!(history
        .send(GetEvents::<InterfoldEvent>::new())
        .await?
        .is_empty());
    Ok(())
}

#[actix::test]
async fn cached_verdict_persists_under_the_current_resume_event() -> Result<()> {
    use e3_events::{Insert, InsertBatch, StoreKeys, WithAggregateId};
    let (mut aggregator, history, _) = small_aggregator().await?;
    collect(&mut aggregator, 5..15)?;
    aggregator.effects_enabled = false;
    let outcome = c6_completion(&aggregator, BTreeSet::new());
    let aggregate = outcome.get_ctx().aggregate_id();
    aggregator.handle_c6_verification_complete(outcome)?;

    let store = InMemStore::new(false).start();
    let data = DataStore::from_in_mem(&store);
    let state_repo = Repository::new(data.scope("state"));
    let recovery_repo = Repository::new(data.scope("recovery"));
    state_repo.write_sync(&aggregator.state.try_get()?).await?;
    recovery_repo
        .write_sync(&aggregator.recovery.try_get()?)
        .await?;
    aggregator.state = state_repo.load().await?;
    aggregator.recovery = recovery_repo.load().await?;
    // Simulate a later committed snapshot. Writes under the old verdict must be rejected.
    store
        .send(InsertBatch::new(vec![Insert::new(
            StoreKeys::aggregate_seq(aggregate).into_bytes(),
            10u64.to_le_bytes().to_vec(),
        )]))
        .await??;
    aggregator.effects_enabled = true;
    // EffectsEnabled belongs to aggregate zero, not this chain. Resume through a new E3 event.
    aggregator.resume_in_flight_work(test_ctx(EffectsEnabled::new()))?;
    let resumed = next_event(&history).await?;
    let InterfoldEventData::PlaintextVerificationResumed(payload) = resumed.into_data() else {
        panic!("expected a fresh C6 resume event")
    };
    assert_eq!(
        InterfoldEventData::from(payload.clone()).get_aggregate_id(),
        aggregate
    );
    let resume_context =
        EventContext::<Unsequenced>::from(InterfoldEventData::from(payload.clone())).sequence(20);
    aggregator.handle_c6_resume(TypedEvent::new(payload, resume_context))?;
    assert!(matches!(
        state_repo.read().await?,
        Some(ThresholdPlaintextAggregatorState::Computing(_))
    ));
    assert_eq!(
        recovery_repo.read().await?.unwrap().honest_c6_proofs.len(),
        10
    );
    Ok(())
}

#[actix::test]
async fn a_bad_share_for_the_second_ciphertext_does_not_reserve_a_slot() -> Result<()> {
    let (mut aggregator, _history, _) = small_aggregator().await?;
    let ec = test_ctx(EffectsEnabled::new());
    let mut collecting = Collecting::try_from(aggregator.state.try_get()?)?;
    collecting.ciphertext_output = test_ciphertexts();
    aggregator.state = test_persistable(ThresholdPlaintextAggregatorState::Collecting(collecting));
    let ids: Vec<u32> = (6..=15).collect();
    for party in 5..15 {
        let (mut shares, proofs) = share_with_matching_commitment_in_set(
            &aggregator.e3_id,
            party,
            &test_ciphertexts(),
            &ids,
        );
        if party == 5 {
            shares[1] = ArcBytes::from_bytes(&[0]);
        }
        aggregator.add_share(party, shares, proofs, &ids, &ec)?;
    }
    let collecting = Collecting::try_from(aggregator.state.try_get()?)?;
    assert!(!collecting.shares.contains_key(&5));
    let (shares, proofs) =
        share_with_matching_commitment_in_set(&aggregator.e3_id, 5, &test_ciphertexts(), &ids);
    aggregator.add_share(5, shares, proofs, &ids, &ec)?;
    let batch = VerifyingC6::try_from(aggregator.state.try_get()?)?;
    assert!(batch.shares.contains_key(&5));
    assert!(batch.shares.contains_key(&14));
    let replacement = c6_completion(&aggregator, BTreeSet::new());
    aggregator.handle_c6_verification_complete(replacement)?;
    assert!(matches!(
        aggregator.state.get(),
        Some(ThresholdPlaintextAggregatorState::Computing(_))
    ));
    Ok(())
}

#[actix::test]
async fn expulsion_of_a_designated_decryptor_starts_a_fresh_round() -> Result<()> {
    let (mut aggregator, history, _) = small_aggregator().await?;
    let ec = test_ctx(EffectsEnabled::new());
    aggregator.state.try_mutate(&ec, |mut state| {
        let ThresholdPlaintextAggregatorState::Collecting(collecting) = &mut state else {
            bail!("expected Collecting");
        };
        collecting.expected_decryptors = (6u32..=15).collect();
        collecting
            .shares
            .insert(6, vec![ArcBytes::from_bytes(&[1])]);
        Ok(state)
    })?;
    aggregator.handle_member_expelled(5, &ec)?;
    let collecting = Collecting::try_from(aggregator.state.try_get()?)?;
    assert!(collecting.shares.is_empty());
    assert!(collecting.rejected_parties.contains(&5));
    assert_eq!(
        collecting.expected_decryptors,
        (7u32..=16).collect::<Vec<_>>()
    );
    actix::clock::sleep(std::time::Duration::from_millis(20)).await;
    let events = history.send(GetEvents::<InterfoldEvent>::new()).await?;
    assert!(events.iter().any(|event| matches!(
        event.get_data(),
        InterfoldEventData::FreshDecryptionRound(_)
    )));
    Ok(())
}

async fn deliver_resume(
    aggregator: &mut ThresholdPlaintextAggregator,
    history: &Addr<HistoryCollector<InterfoldEvent>>,
) -> Result<()> {
    loop {
        let event = next_event(history).await?;
        if let InterfoldEventData::PlaintextVerificationResumed(payload) = event.get_data() {
            return aggregator.handle_c6_resume(event.to_typed_event(payload.clone()));
        }
    }
}
