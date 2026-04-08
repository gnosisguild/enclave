// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Actor;
use alloy::primitives::{Address, FixedBytes, I256, U256};
use alloy::signers::local::PrivateKeySigner;
use anyhow::{bail, Result};
use e3_bfv_client::decode_bytes_to_vec_u64;
use e3_ciphernode_builder::{CiphernodeBuilder, EventSystem};
use e3_config::BBPath;
use e3_crypto::Cipher;
use e3_events::{
    prelude::*, BusHandle, CiphertextOutputPublished, CommitteeFinalized, ComputeRequestKind,
    ComputeResponseKind, ConfigurationUpdated, E3Requested, E3id, EffectsEnabled, EnclaveEvent,
    EnclaveEventData, EventType, GetEvents, HistoryCollector, OperatorActivationChanged,
    OrderedSet, PkAggregationProofPending, PkAggregationProofRequest, PlaintextAggregated,
    ProofType, Seed, TakeEvents, TicketBalanceUpdated, VerificationKind, ZkRequest, ZkResponse,
};
use e3_fhe_params::DEFAULT_BFV_PRESET;
use e3_fhe_params::{build_pair_for_preset, create_deterministic_crp_from_default_seed};
use e3_fhe_params::{encode_bfv_params, BfvParamSet, BfvPreset};
use e3_multithread::{Multithread, MultithreadReport, ToReport};
use e3_net::events::{GossipData, NetEvent};
use e3_net::NetEventTranslator;
use e3_polynomial::CrtPolynomial;
use e3_sortition::{calculate_buffer_size, RegisteredNode, ScoreSortition, Ticket};
use e3_test_helpers::ciphernode_system::{
    CiphernodeHistory, CiphernodeSystem, CiphernodeSystemBuilder,
};
use e3_test_helpers::{
    create_seed_from_u64, create_shared_rng_from_u64, find_bb, with_tracing, AddToCommittee,
};
use e3_trbfv::helpers::calculate_error_size;
use e3_trbfv::{TrBFVRequest, TrBFVResponse};
use e3_utils::utility_types::ArcBytes;
use e3_utils::{colorize, rand_eth_addr, Color};
use e3_zk_helpers::{compute_modulus_bit, compute_threshold_pk_commitment};
use e3_zk_prover::test_utils::get_tempdir;
use e3_zk_prover::{ProofRequestActor, VersionInfo, ZkBackend};
use fhe::bfv::PublicKey;
use fhe::bfv::SecretKey;
use fhe::mbfv::{AggregateIter, PublicKeyShare};
use fhe_traits::{DeserializeParametrized, Serialize};
use num_bigint::BigUint;
use rand::rngs::OsRng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::time::{Duration, Instant};
use std::{fs, path::PathBuf, sync::Arc};
use tokio::{
    sync::{broadcast, mpsc},
    time::sleep,
};

/// Create a ZkBackend for integration tests.
/// If a local bb binary is found, uses it with fixture files (fast path).
/// Otherwise, calls `ensure_installed()` to download bb + circuits (CI path).
async fn setup_test_zk_backend() -> (ZkBackend, tempfile::TempDir) {
    let temp = get_tempdir().unwrap();
    let temp_path = temp.path();
    let noir_dir = temp_path.join("noir");
    let bb_binary = noir_dir.join("bin").join("bb");
    let circuits_dir = noir_dir.join("circuits");
    let work_dir = noir_dir.join("work").join("test_node");

    if let Some(bb) = find_bb().await {
        tokio::fs::create_dir_all(bb_binary.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::create_dir_all(&circuits_dir).await.unwrap();
        tokio::fs::create_dir_all(&work_dir).await.unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&bb, &bb_binary).unwrap();
        #[cfg(not(unix))]
        compile_error!("Integration tests require unix symlink support");

        let circuits_build_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("circuits")
            .join("bin");
        let dkg_target = circuits_build_root.join("dkg").join("target");
        let threshold_target = circuits_build_root.join("threshold").join("target");
        let wrapper_dkg_target = circuits_build_root
            .join("recursive_aggregation")
            .join("wrapper")
            .join("dkg")
            .join("target");
        let wrapper_threshold_target = circuits_build_root
            .join("recursive_aggregation")
            .join("wrapper")
            .join("threshold")
            .join("target");
        let fold_target = circuits_build_root
            .join("recursive_aggregation")
            .join("fold")
            .join("target");

        // Helper: copy {name}.json + VK artifacts into a destination directory.
        // vk_suffix/vk_hash_suffix select the source VK flavor:
        //   ".vk_noir" / ".vk_noir_hash"       → Recursive variant (inner proofs)
        //   ".vk_recursive" / ".vk_recursive_hash" → Default variant (wrapper/fold proofs)
        //   ".vk" / ".vk_hash"                  → EVM variant
        async fn copy_circuit(
            src_dir: &std::path::Path,
            dst_dir: &std::path::Path,
            name: &str,
            vk_suffix: &str,
            vk_hash_suffix: &str,
        ) {
            tokio::fs::create_dir_all(dst_dir).await.unwrap();
            tokio::fs::copy(
                src_dir.join(format!("{name}.json")),
                dst_dir.join(format!("{name}.json")),
            )
            .await
            .unwrap();
            tokio::fs::copy(
                src_dir.join(format!("{name}{vk_suffix}")),
                dst_dir.join(format!("{name}.vk")),
            )
            .await
            .unwrap();
            tokio::fs::copy(
                src_dir.join(format!("{name}{vk_hash_suffix}")),
                dst_dir.join(format!("{name}.vk_hash")),
            )
            .await
            .unwrap();
        }

        // ── recursive/ variant (inner/base proofs, uses .vk_noir) ──────────
        // Tests use insecure params, so fixtures go under insecure-512/
        let preset_dir = circuits_dir.join("insecure-512");

        let rv = preset_dir.join("recursive");

        // T0 (pk)
        copy_circuit(
            &dkg_target,
            &rv.join("dkg/pk"),
            "pk",
            ".vk_noir",
            ".vk_noir_hash",
        )
        .await;
        // C1 (pk_generation)
        copy_circuit(
            &threshold_target,
            &rv.join("threshold/pk_generation"),
            "pk_generation",
            ".vk_noir",
            ".vk_noir_hash",
        )
        .await;
        // C2a (sk_share_computation)
        copy_circuit(
            &dkg_target,
            &rv.join("dkg/sk_share_computation"),
            "sk_share_computation",
            ".vk_noir",
            ".vk_noir_hash",
        )
        .await;
        // C2b (e_sm_share_computation)
        copy_circuit(
            &dkg_target,
            &rv.join("dkg/e_sm_share_computation"),
            "e_sm_share_computation",
            ".vk_noir",
            ".vk_noir_hash",
        )
        .await;
        // C3 (share_encryption)
        copy_circuit(
            &dkg_target,
            &rv.join("dkg/share_encryption"),
            "share_encryption",
            ".vk_noir",
            ".vk_noir_hash",
        )
        .await;
        // C4 (dkg/share_decryption)
        copy_circuit(
            &dkg_target,
            &rv.join("dkg/share_decryption"),
            "share_decryption",
            ".vk_noir",
            ".vk_noir_hash",
        )
        .await;
        // C5 (pk_aggregation)
        copy_circuit(
            &threshold_target,
            &rv.join("threshold/pk_aggregation"),
            "pk_aggregation",
            ".vk_noir",
            ".vk_noir_hash",
        )
        .await;
        // C6 (threshold/share_decryption)
        copy_circuit(
            &threshold_target,
            &rv.join("threshold/share_decryption"),
            "share_decryption",
            ".vk_noir",
            ".vk_noir_hash",
        )
        .await;
        // C7 (decrypted_shares_aggregation)
        copy_circuit(
            &threshold_target,
            &rv.join("threshold/decrypted_shares_aggregation"),
            "decrypted_shares_aggregation",
            ".vk_noir",
            ".vk_noir_hash",
        )
        .await;

        // ── default/ variant (wrapper & fold proofs, uses .vk_recursive) ───

        let dv = preset_dir.join("default");

        // DKG wrapper circuits
        let dkg_wrapper_base = dv.join("recursive_aggregation/wrapper/dkg");
        copy_circuit(
            &wrapper_dkg_target,
            &dkg_wrapper_base.join("pk"),
            "pk",
            ".vk_recursive",
            ".vk_recursive_hash",
        )
        .await;
        copy_circuit(
            &wrapper_dkg_target,
            &dkg_wrapper_base.join("share_computation"),
            "share_computation",
            ".vk_recursive",
            ".vk_recursive_hash",
        )
        .await;
        copy_circuit(
            &wrapper_dkg_target,
            &dkg_wrapper_base.join("share_encryption"),
            "share_encryption",
            ".vk_recursive",
            ".vk_recursive_hash",
        )
        .await;
        copy_circuit(
            &wrapper_dkg_target,
            &dkg_wrapper_base.join("share_decryption"),
            "share_decryption",
            ".vk_recursive",
            ".vk_recursive_hash",
        )
        .await;

        // Threshold wrapper circuits
        let threshold_wrapper_base = dv.join("recursive_aggregation/wrapper/threshold");
        copy_circuit(
            &wrapper_threshold_target,
            &threshold_wrapper_base.join("pk_generation"),
            "pk_generation",
            ".vk_recursive",
            ".vk_recursive_hash",
        )
        .await;
        copy_circuit(
            &wrapper_threshold_target,
            &threshold_wrapper_base.join("pk_aggregation"),
            "pk_aggregation",
            ".vk_recursive",
            ".vk_recursive_hash",
        )
        .await;
        copy_circuit(
            &wrapper_threshold_target,
            &threshold_wrapper_base.join("share_decryption"),
            "share_decryption",
            ".vk_recursive",
            ".vk_recursive_hash",
        )
        .await;
        copy_circuit(
            &wrapper_threshold_target,
            &threshold_wrapper_base.join("decrypted_shares_aggregation"),
            "decrypted_shares_aggregation",
            ".vk_recursive",
            ".vk_recursive_hash",
        )
        .await;

        // Fold circuit (default variant)
        copy_circuit(
            &fold_target,
            &dv.join("recursive_aggregation/fold"),
            "fold",
            ".vk_recursive",
            ".vk_recursive_hash",
        )
        .await;

        // ── evm/ variant (on-chain verification: C5, C7, fold) ───────────

        let ev = preset_dir.join("evm");

        // C5 (pk_aggregation) — EVM-targeted
        copy_circuit(
            &threshold_target,
            &ev.join("threshold/pk_aggregation"),
            "pk_aggregation",
            ".vk",
            ".vk_hash",
        )
        .await;
        // C7 (decrypted_shares_aggregation) — EVM-targeted
        copy_circuit(
            &threshold_target,
            &ev.join("threshold/decrypted_shares_aggregation"),
            "decrypted_shares_aggregation",
            ".vk",
            ".vk_hash",
        )
        .await;
        // Fold circuit — final EVM fold
        copy_circuit(
            &fold_target,
            &ev.join("recursive_aggregation/fold"),
            "fold",
            ".vk",
            ".vk_hash",
        )
        .await;

        let backend = ZkBackend::new(BBPath::Default(bb_binary), circuits_dir, work_dir);

        // `CiphernodeBuilder` calls `ensure_installed()`, which deletes `circuits_dir` and downloads
        // the release tarball whenever `version.json` does not record the pinned bb/circuits
        // versions. That would wipe the fixture tree we just copied from `circuits/bin/`.
        let mut version_info = VersionInfo::default();
        version_info.bb_version = Some(backend.config.required_bb_version.clone());
        version_info.circuits_version = Some(backend.config.required_circuits_version.clone());
        version_info
            .save(&backend.version_file())
            .await
            .expect("write noir/version.json for integration ZK fixtures");

        (backend, temp)
    } else {
        println!("bb binary not found locally, downloading via ensure_installed()...");
        let backend = ZkBackend::new(BBPath::Default(bb_binary), circuits_dir, work_dir);
        backend
            .ensure_installed()
            .await
            .expect("Failed to download and install ZK backend");
        (backend, temp)
    }
}

pub fn save_snapshot(file_name: &str, bytes: &[u8]) {
    println!("### WRITING SNAPSHOT TO `{file_name}` ###");
    fs::write(format!("tests/{file_name}"), bytes).unwrap();
}

/// Compute placeholder scores for a committee.
/// Uses ticket_id=0 for each address with the given e3_id and seed.
fn compute_committee_scores(committee: &[String], e3_id: &E3id, seed: Seed) -> Vec<String> {
    use e3_sortition::hash_to_score;
    committee
        .iter()
        .map(|addr| {
            let address: Address = addr.parse().unwrap();
            let score = hash_to_score(address, 0, e3_id.clone(), seed);
            U256::from_be_slice(&score.to_bytes_be()).to_string()
        })
        .collect()
}

/// Determines the committee for a given E3 request using deterministic sortition.
///
/// This function runs the same sortition algorithm that the ciphernodes use internally,
/// ensuring the test committee matches what the nodes will compute.
///
/// # Arguments
/// * `e3_id` - The E3 computation ID
/// * `seed` - The random seed for sortition
/// * `threshold_m` - Minimum nodes required for decryption
/// * `threshold_n` - Committee size
/// * `registered_addrs` - List of node addresses eligible for selection
/// * `collector_addr` - Address of the collector node (for validation)
///
/// # Returns
/// A tuple of (committee_addresses, committee_scores, buffer_addresses)
fn determine_committee(
    e3_id: &E3id,
    seed: Seed,
    threshold_m: usize,
    threshold_n: usize,
    registered_addrs: &[String],
    collector_addr: &str,
) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
    let buffer = calculate_buffer_size(threshold_m, threshold_n);
    let total_selection_size = threshold_n + buffer;

    // Calculate tickets based on the same balance/ticket_price ratio as production
    // ticket_price = 10_000_000, balance = 1_000_000_000
    // => num_tickets = 1_000_000_000 / 10_000_000 = 100 tickets per node
    const TICKET_PRICE: u64 = 10_000_000;
    const BALANCE: u64 = 1_000_000_000;
    let num_tickets = BALANCE / TICKET_PRICE;

    let registered_nodes: Vec<RegisteredNode> = registered_addrs
        .iter()
        .map(|addr| {
            let address: Address = addr.parse().unwrap();
            let tickets: Vec<Ticket> = (0..num_tickets)
                .map(|ticket_id| Ticket { ticket_id })
                .collect();
            RegisteredNode { address, tickets }
        })
        .collect();

    let winners = ScoreSortition::new(total_selection_size).get_committee(
        e3_id.clone(),
        seed,
        &registered_nodes,
    )?;

    let committee: Vec<String> = winners
        .iter()
        .take(threshold_n)
        .map(|w| w.address.to_string())
        .collect();

    let committee_scores: Vec<String> = winners
        .iter()
        .take(threshold_n)
        .map(|w| U256::from_be_slice(&w.score.to_bytes_be()).to_string())
        .collect();

    let buffer_nodes: Vec<String> = winners
        .iter()
        .skip(threshold_n)
        .map(|w| w.address.to_string())
        .collect();

    for addr in &committee {
        if addr.eq_ignore_ascii_case(collector_addr) {
            bail!(
                "Collector node was selected in committee. \
                 This should never happen as collector should not be registered for sortition.\n\
                 Collector: {}\n\
                 Registered nodes: {}",
                collector_addr,
                registered_addrs.len()
            );
        }
    }

    Ok((committee, committee_scores, buffer_nodes))
}

fn find_node_index_by_address(nodes: &CiphernodeSystem, address: &str) -> Result<usize> {
    for (index, node) in nodes.iter().enumerate() {
        if node.address().eq_ignore_ascii_case(address) {
            return Ok(index);
        }
    }

    bail!("Could not find node index for address {address}");
}

async fn expect_node_events_with_timeouts(
    nodes: &CiphernodeSystem,
    index: usize,
    expected: &[&str],
    total_to: Duration,
    per_evt_to: Duration,
) -> Result<CiphernodeHistory> {
    let h = nodes
        .take_history_with_timeouts(index, expected.len(), Some(total_to), Some(per_evt_to))
        .await
        .map_err(|e| anyhow::anyhow!("FAILURE on node {index}: {expected:?} : {e}"))?;

    println!(
        "node {index} >> {:?} == {:?}",
        h.event_types(),
        expected.to_vec()
    );
    h.expect(expected.to_vec());
    Ok(h)
}

fn project_history<F>(history: &[EnclaveEvent], mut projector: F) -> Vec<&'static str>
where
    F: FnMut(&EnclaveEventData) -> Option<&'static str>,
{
    history
        .iter()
        .filter_map(|event| projector(event.get_data()))
        .collect()
}

fn count_projected_events(projected: &[&str], event_type: &str) -> usize {
    projected.iter().filter(|seen| **seen == event_type).count()
}

fn publickey_aggregator_marker(data: &EnclaveEventData, e3_id: &E3id) -> Option<&'static str> {
    match data {
        EnclaveEventData::CommitteeFinalized(data) if data.e3_id == *e3_id => {
            Some("CommitteeFinalized")
        }
        EnclaveEventData::CiphernodeSelected(data) if data.e3_id == *e3_id => {
            Some("CiphernodeSelected")
        }
        EnclaveEventData::AggregatorChanged(data) if data.e3_id == *e3_id && data.is_aggregator => {
            Some("AggregatorChanged")
        }
        EnclaveEventData::KeyshareCreated(data) if data.e3_id == *e3_id => Some("KeyshareCreated"),
        EnclaveEventData::ShareVerificationDispatched(data)
            if data.e3_id == *e3_id && data.kind == VerificationKind::PkGenerationProofs =>
        {
            Some("ShareVerificationDispatched")
        }
        EnclaveEventData::CommitmentConsistencyCheckRequested(data)
            if data.e3_id == *e3_id && data.kind == VerificationKind::PkGenerationProofs =>
        {
            Some("CommitmentConsistencyCheckRequested")
        }
        EnclaveEventData::CommitmentConsistencyCheckComplete(data)
            if data.e3_id == *e3_id && data.kind == VerificationKind::PkGenerationProofs =>
        {
            Some("CommitmentConsistencyCheckComplete")
        }
        EnclaveEventData::ProofVerificationPassed(data)
            if data.e3_id == *e3_id && data.proof_type == ProofType::C1PkGeneration =>
        {
            Some("ProofVerificationPassed")
        }
        EnclaveEventData::ShareVerificationComplete(data)
            if data.e3_id == *e3_id && data.kind == VerificationKind::PkGenerationProofs =>
        {
            Some("ShareVerificationComplete")
        }
        EnclaveEventData::PkAggregationProofPending(data) if data.e3_id == *e3_id => {
            Some("PkAggregationProofPending")
        }
        EnclaveEventData::PkAggregationProofSigned(data) if data.e3_id == *e3_id => {
            Some("PkAggregationProofSigned")
        }
        EnclaveEventData::DKGRecursiveAggregationComplete(data) if data.e3_id == *e3_id => {
            Some("DKGRecursiveAggregationComplete")
        }
        EnclaveEventData::PublicKeyAggregated(data) if data.e3_id == *e3_id => {
            Some("PublicKeyAggregated")
        }
        _ => None,
    }
}

fn plaintext_aggregator_marker(data: &EnclaveEventData, e3_id: &E3id) -> Option<&'static str> {
    match data {
        EnclaveEventData::CiphertextOutputPublished(data) if data.e3_id == *e3_id => {
            Some("CiphertextOutputPublished")
        }
        EnclaveEventData::DecryptionshareCreated(data) if data.e3_id == *e3_id => {
            Some("DecryptionshareCreated")
        }
        EnclaveEventData::ShareVerificationDispatched(data)
            if data.e3_id == *e3_id && data.kind == VerificationKind::ThresholdDecryptionProofs =>
        {
            Some("ShareVerificationDispatched")
        }
        EnclaveEventData::CommitmentConsistencyCheckRequested(data)
            if data.e3_id == *e3_id && data.kind == VerificationKind::ThresholdDecryptionProofs =>
        {
            Some("CommitmentConsistencyCheckRequested")
        }
        EnclaveEventData::CommitmentConsistencyCheckComplete(data)
            if data.e3_id == *e3_id && data.kind == VerificationKind::ThresholdDecryptionProofs =>
        {
            Some("CommitmentConsistencyCheckComplete")
        }
        EnclaveEventData::ComputeRequest(data)
            if data.e3_id == *e3_id
                && matches!(
                    &data.request,
                    ComputeRequestKind::Zk(ZkRequest::VerifyShareProofs(_))
                        | ComputeRequestKind::TrBFV(TrBFVRequest::CalculateThresholdDecryption(_))
                        | ComputeRequestKind::Zk(ZkRequest::DecryptedSharesAggregation(_))
                        | ComputeRequestKind::Zk(ZkRequest::FoldProofs { .. })
                ) =>
        {
            Some("ComputeRequest")
        }
        EnclaveEventData::ComputeResponse(data)
            if data.e3_id == *e3_id
                && matches!(
                    &data.response,
                    ComputeResponseKind::Zk(ZkResponse::VerifyShareProofs(_))
                        | ComputeResponseKind::TrBFV(TrBFVResponse::CalculateThresholdDecryption(
                            _
                        ))
                        | ComputeResponseKind::Zk(ZkResponse::DecryptedSharesAggregation(_))
                        | ComputeResponseKind::Zk(ZkResponse::FoldProofs(_))
                ) =>
        {
            Some("ComputeResponse")
        }
        EnclaveEventData::ProofVerificationPassed(data)
            if data.e3_id == *e3_id && data.proof_type == ProofType::C6ThresholdShareDecryption =>
        {
            Some("ProofVerificationPassed")
        }
        EnclaveEventData::ShareVerificationComplete(data)
            if data.e3_id == *e3_id && data.kind == VerificationKind::ThresholdDecryptionProofs =>
        {
            Some("ShareVerificationComplete")
        }
        EnclaveEventData::AggregationProofPending(data) if data.e3_id == *e3_id => {
            Some("AggregationProofPending")
        }
        EnclaveEventData::AggregationProofSigned(data) if data.e3_id == *e3_id => {
            Some("AggregationProofSigned")
        }
        EnclaveEventData::PlaintextAggregated(data) if data.e3_id == *e3_id => {
            Some("PlaintextAggregated")
        }
        _ => None,
    }
}

async fn setup_score_sortition_environment(
    bus: &BusHandle,
    eth_addrs: &Vec<String>,
    chain_id: u64,
) -> Result<()> {
    bus.publish_without_context(ConfigurationUpdated {
        parameter: "ticketPrice".to_string(),
        old_value: U256::ZERO,
        new_value: U256::from(10_000_000u64),
        chain_id,
    })?;

    let mut adder = AddToCommittee::new(bus, chain_id);
    for addr in eth_addrs {
        adder.add(addr).await?;

        bus.publish_without_context(TicketBalanceUpdated {
            operator: addr.clone(),
            delta: I256::try_from(1_000_000_000u64).unwrap(),
            new_balance: U256::from(1_000_000_000u64),
            reason: FixedBytes::ZERO,
            chain_id,
        })?;

        bus.publish_without_context(OperatorActivationChanged {
            operator: addr.clone(),
            active: true,
            chain_id,
        })?;
    }

    Ok(())
}

#[derive(Default)]
struct Report {
    inner: Vec<(String, Duration)>,
}

fn repeat(ch: char, num: usize) -> String {
    let mut s = String::new();
    while s.len() < num {
        s.push(ch);
    }
    s
}

impl Report {
    pub fn push(&mut self, repo: (&str, Duration)) {
        let (label, dur) = repo;
        self.show(label);
        self.inner.push((label.to_owned(), dur));
    }

    pub fn show(&self, label: &str) {
        println!(
            "\n\n {}\n {}{}{}\n {}\n",
            colorize(repeat('#', label.len() + 6), Color::Yellow),
            colorize("## ", Color::Yellow),
            colorize(label.to_uppercase(), Color::White),
            colorize(" ##", Color::Yellow),
            colorize(repeat('#', label.len() + 6), Color::Yellow),
        );
    }

    pub fn serialize(&self) -> String {
        let max_key_len = self.inner.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

        self.inner
            .iter()
            .map(|(key, duration)| {
                format!(
                    "{:width$}: {:.3}s",
                    key,
                    duration.as_secs_f64(),
                    width = max_key_len
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Test trbfv
#[actix::test]
#[serial_test::serial]
async fn test_trbfv_actor() -> Result<()> {
    let mut report = Report::default();
    report.push(("Starting trbfv actor test", Duration::from_secs(0)));
    let whole_test = Instant::now();
    let _guard = with_tracing("info");

    // NOTE: Here we are trying to make it as clear as possible as to what is going on so attempting to
    // avoid over abstracting test helpers and favouring straight forward single descriptive
    // functions alongside explanations

    ///////////////////////////////////////////////////////////////////////////////////
    // 1. Setup ThresholdKeyshare system
    //
    //   - E3Router
    //   - ThresholdKeyshare
    //   - Multithread actor
    //   - 20 nodes (so as to check for some nodes not getting selected)
    //   - Loopback libp2p simulation
    ///////////////////////////////////////////////////////////////////////////////////

    let setup = Instant::now();

    // Create rng
    let rng = create_shared_rng_from_u64(42);

    // Create "trigger" bus
    let system = EventSystem::new().with_fresh_bus();
    let bus = system.handle()?.enable("test");

    // Parameters (128bits of security)
    let params_raw = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();

    // Encoded Params
    let params = ArcBytes::from_bytes(&encode_bfv_params(&params_raw.clone()));

    // round information
    let threshold_m = 1;
    let threshold_n = 3;
    let esi_per_ct = 1;

    // WARNING: INSECURE SECURITY PARAMETER LAMBDA.
    // This is just for INSECURE parameter set.
    // This is not secure and should not be used in production.
    // For production use lambda = 80.
    let lambda = 2;

    let seed = create_seed_from_u64(123);
    let error_size = ArcBytes::from_bytes(&BigUint::to_bytes_be(&calculate_error_size(
        params_raw.clone(),
        threshold_n,
        threshold_m,
        lambda,
    )?));

    // Cipher
    let cipher = Arc::new(Cipher::from_password("I am the music man.").await?);

    // Actor system setup
    // Seems like you cannot send more than one job at a time to rayon
    let concurrent_jobs = 1;
    let max_threadroom = Multithread::get_max_threads_minus(1);
    let task_pool = Multithread::create_taskpool(max_threadroom, concurrent_jobs);
    let multithread_report = MultithreadReport::new(max_threadroom, concurrent_jobs).start();

    // Setup ZK backend for proof generation/verification
    let (zk_backend, _zk_temp) = setup_test_zk_backend().await;

    let nodes = CiphernodeSystemBuilder::new()
        // All nodes run the same binary under the aggregator-committee model.
        // Node 0 stays an observer only because it is excluded from sortition registration.
        // Adding 20 total nodes: 3 for committee + 3 buffer = 6 selected, 14 unselected
        .add_group(1, || async {
            let addr = rand_eth_addr(&rng);
            println!("Building collector {}!", addr);
            CiphernodeBuilder::new(rng.clone(), cipher.clone())
                .testmode_with_history()
                .with_shared_taskpool(&task_pool)
                .with_multithread_concurrent_jobs(concurrent_jobs)
                .with_shared_multithread_report(&multithread_report)
                .with_trbfv()
                .with_zkproof(zk_backend.clone())
                .testmode_with_signer(PrivateKeySigner::random())
                .with_pubkey_aggregation()
                .with_sortition_score()
                .with_threshold_plaintext_aggregation()
                .testmode_with_forked_bus(bus.event_bus())
                .testmode_ignore_address_check()
                .with_logging()
                .build()
                .await
        })
        .add_group(19, || async {
            let addr = rand_eth_addr(&rng);
            println!("Building normal {}", &addr);
            CiphernodeBuilder::new(rng.clone(), cipher.clone())
                .testmode_with_history()
                .with_shared_taskpool(&task_pool)
                .with_multithread_concurrent_jobs(concurrent_jobs)
                .with_shared_multithread_report(&multithread_report)
                .with_trbfv()
                .with_zkproof(zk_backend.clone())
                .testmode_with_signer(PrivateKeySigner::random())
                .with_pubkey_aggregation()
                .with_sortition_score()
                .with_threshold_plaintext_aggregation()
                .testmode_with_forked_bus(bus.event_bus())
                .testmode_ignore_address_check()
                .with_logging()
                .build()
                .await
        })
        .simulate_libp2p()
        .build()
        .await?;

    report.push(("Setup completed", setup.elapsed()));

    let committee_setup = Instant::now();
    let chain_id = 1u64;

    // Only register nodes 1-19 in sortition (exclude collector at index 0).
    // This ensures the collector is never selected, making the test deterministic.
    // The collector node will observe events as a non-participant.
    let collector_addr = nodes.get(0).unwrap().address();
    let eth_addrs: Vec<String> = nodes
        .iter()
        .skip(1) // Skip the collector node
        .map(|n| n.address())
        .collect();

    println!(
        "Test setup: {} registered nodes, {} threshold, collector (observer): {}",
        eth_addrs.len(),
        threshold_n,
        collector_addr
    );

    setup_score_sortition_environment(&bus, &eth_addrs, chain_id).await?;

    // Flush all events
    nodes.flush_all_history(10000).await?;

    report.push(("Committee Setup Completed", committee_setup.elapsed()));

    ///////////////////////////////////////////////////////////////////////////////////
    // 2. Trigger E3Requested
    //
    //   - m=1.
    //   - n=3
    //   - lambda=2
    //   - error_size -> calculate using calculate_error_size
    //   - esi_per_ciphertext = 1
    ///////////////////////////////////////////////////////////////////////////////////

    // Prepare round
    let e3_requested_timer = Instant::now();
    // Trigger actor DKG
    let e3_id = E3id::new("0", 1);

    let proof_aggregation_enabled = false;

    let e3_requested = E3Requested {
        e3_id: e3_id.clone(),
        threshold_m,
        threshold_n,
        seed: seed.clone(),
        error_size,
        esi_per_ct: esi_per_ct as usize,
        params_preset: DEFAULT_BFV_PRESET,
        params,
        proof_aggregation_enabled,
    };

    bus.publish_without_context(e3_requested)?;

    sleep(Duration::from_millis(500)).await;

    let (committee, committee_scores, buffer_nodes) = determine_committee(
        &e3_id,
        seed,
        threshold_m,
        threshold_n,
        &eth_addrs,
        &collector_addr,
    )?;

    report.show(&format!(
        "Committee selected: {} nodes, {} buffer nodes",
        committee.len(),
        buffer_nodes.len()
    ));

    let active_aggregator_addr = committee
        .first()
        .cloned()
        .expect("committee should have an active aggregator");
    let active_aggregator_index = find_node_index_by_address(&nodes, &active_aggregator_addr)?;

    println!(
        "Resolved active aggregator: node index {} ({})",
        active_aggregator_index, active_aggregator_addr
    );

    nodes.expect_events(&["E3Requested"]).await?;

    bus.publish_without_context(CommitteeFinalized {
        e3_id: e3_id.clone(),
        committee: committee.clone(),
        scores: committee_scores,
        chain_id,
    })?;

    let committee_finalized_timer = Instant::now();

    nodes.expect_events(&["CommitteeFinalized"]).await?;

    report.push((
        "Committee Finalization Complete",
        committee_finalized_timer.elapsed(),
    ));

    // Node 0 is a non-committee observer. It only sees bus-global events and the forwardable
    // gossip events from the active aggregator flow.
    let shares_to_pubkey_agg_timer = Instant::now();
    const KS3: [&str; 3] = ["KeyshareCreated"; 3];
    const DKG3: [&str; 3] = ["DKGRecursiveAggregationComplete"; 3];
    const ACTIVE_AGGREGATOR_C1_C5: [&str; 9] = [
        "ShareVerificationDispatched",
        "CommitmentConsistencyCheckRequested",
        "CommitmentConsistencyCheckComplete",
        "ProofVerificationPassed",
        "ProofVerificationPassed",
        "ProofVerificationPassed",
        "ShareVerificationComplete",
        "PkAggregationProofPending",
        "PkAggregationProofSigned",
    ];

    let mut expected_events: Vec<&'static str> = vec!["AggregatorChanged"];
    if proof_aggregation_enabled {
        expected_events.extend_from_slice(&KS3);
        expected_events.extend_from_slice(&DKG3);
    } else {
        expected_events.extend_from_slice(&DKG3);
        expected_events.extend_from_slice(&KS3);
    }
    expected_events.push("PublicKeyAggregated");
    let h = expect_node_events_with_timeouts(
        &nodes,
        0,
        &expected_events,
        Duration::from_secs(5000),
        Duration::from_secs(5000),
    )
    .await?;

    let active_aggregator_history = nodes.get_history(active_aggregator_index).await?;
    let active_aggregator_pubkey_history_len = active_aggregator_history.len();
    let mut expected_active_aggregator_pubkey_events = vec![
        "CommitteeFinalized",
        "CiphernodeSelected",
        "AggregatorChanged",
    ];
    if proof_aggregation_enabled {
        expected_active_aggregator_pubkey_events.extend_from_slice(&KS3);
    } else {
        expected_active_aggregator_pubkey_events.extend_from_slice(&DKG3);
        expected_active_aggregator_pubkey_events.extend_from_slice(&KS3);
    }
    expected_active_aggregator_pubkey_events.extend_from_slice(&ACTIVE_AGGREGATOR_C1_C5);
    if proof_aggregation_enabled {
        expected_active_aggregator_pubkey_events.extend_from_slice(&DKG3);
    }
    expected_active_aggregator_pubkey_events.push("PublicKeyAggregated");

    // The active aggregator is also a selected committee member, so its node history contains
    // local ThresholdKeyshare DKG work in addition to the public-key aggregation stage. Project
    // only the deterministic pubkey-aggregation signals instead of comparing the whole raw node bus.
    let active_aggregator_pubkey_events = project_history(&active_aggregator_history, |data| {
        publickey_aggregator_marker(data, &e3_id)
    });
    assert_eq!(
        active_aggregator_pubkey_events, expected_active_aggregator_pubkey_events,
        "Unexpected active aggregator public-key flow"
    );

    report.push((
        "ThresholdShares -> PublicKeyAggregated",
        shares_to_pubkey_agg_timer.elapsed(),
    ));

    report.push((
        "E3Request -> PublicKeyAggregated",
        e3_requested_timer.elapsed(),
    ));
    let app_gen_timer = Instant::now();

    // First we get the public key from the collector-visible gossip event.
    println!("Getting public key");
    let Some(pubkey_event) = h.iter().rev().find_map(|event| match event.get_data() {
        EnclaveEventData::PublicKeyAggregated(data) => Some(data.clone()),
        _ => None,
    }) else {
        panic!(
            "Was expecting collector history to contain PublicKeyAggregated, got: {:?}",
            h.event_types()
        );
    };

    let pubkey_bytes = pubkey_event.pubkey.clone();

    let pubkey = PublicKey::from_bytes(&pubkey_bytes, &params_raw)?;

    println!("Generating inputs this takes some time...");

    // Create the inputs
    let num_votes_per_voter = 3;
    let num_voters = 30;
    let (inputs, numbers) = e3_test_helpers::application::generate_ciphertexts(
        &pubkey,
        params_raw.clone(),
        num_voters,
        num_votes_per_voter,
    );
    report.push(("Application CT Gen", app_gen_timer.elapsed()));

    let running_app_timer = Instant::now();
    println!("Running application to generate outputs...");
    let outputs = e3_test_helpers::application::run_application(
        &inputs,
        params_raw.clone(),
        num_votes_per_voter,
    );
    report.push(("Running FHE Application", running_app_timer.elapsed()));

    let publishing_ct_timer = Instant::now();
    println!("Have outputs. Creating ciphertexts...");
    let ciphertexts = outputs
        .into_iter()
        .map(|ct| ArcBytes::from_bytes(&(*ct).clone().to_bytes()))
        .collect::<Vec<ArcBytes>>();

    // Created the event
    println!("Publishing CiphertextOutputPublished...");
    let ciphertext_published_event = CiphertextOutputPublished {
        ciphertext_output: ciphertexts,
        e3_id: e3_id.clone(),
    };

    bus.publish_without_context(ciphertext_published_event.clone())?;

    println!("CiphertextOutputPublished event has been dispatched!");

    // The collector only sees the shared ciphertext event, gossiped decryption shares, and the
    // final gossiped plaintext output.
    const DS3: [&str; 3] = ["DecryptionshareCreated"; 3];
    let mut expected_events: Vec<&'static str> = vec!["CiphertextOutputPublished"];
    expected_events.extend_from_slice(&DS3);
    expected_events.push("PlaintextAggregated");

    let h = expect_node_events_with_timeouts(
        &nodes,
        0,
        &expected_events,
        Duration::from_secs(1000),
        Duration::from_secs(1000),
    )
    .await?;

    let active_aggregator_history = nodes.get_history(active_aggregator_index).await?;
    const C6_VERIFY_PREFIX: [&str; 19] = [
        "CiphertextOutputPublished",
        "DecryptionshareCreated",
        "DecryptionshareCreated",
        "DecryptionshareCreated",
        "ShareVerificationDispatched",
        "CommitmentConsistencyCheckRequested",
        "CommitmentConsistencyCheckComplete",
        "ComputeRequest",
        "ComputeResponse",
        "ProofVerificationPassed",
        "ProofVerificationPassed",
        "ProofVerificationPassed",
        "ProofVerificationPassed",
        "ProofVerificationPassed",
        "ProofVerificationPassed",
        "ProofVerificationPassed",
        "ProofVerificationPassed",
        "ProofVerificationPassed",
        "ShareVerificationComplete",
    ];
    let active_aggregator_plaintext_events = project_history(
        &active_aggregator_history[active_aggregator_pubkey_history_len..],
        |data| plaintext_aggregator_marker(data, &e3_id),
    );
    assert_eq!(
        &active_aggregator_plaintext_events[..C6_VERIFY_PREFIX.len()],
        C6_VERIFY_PREFIX,
        "Unexpected active aggregator C6 verification prefix"
    );

    let completion_events = &active_aggregator_plaintext_events[C6_VERIFY_PREFIX.len()..];
    let c6_proof_count = threshold_n as usize * num_votes_per_voter;
    let c6_fold_steps = if proof_aggregation_enabled {
        c6_proof_count.saturating_sub(1)
    } else {
        0
    };

    if proof_aggregation_enabled {
        let aggregation_pending_index = completion_events
            .iter()
            .position(|event| *event == "AggregationProofPending")
            .expect("AggregationProofPending should be present");
        let aggregation_signed_index = completion_events
            .iter()
            .position(|event| *event == "AggregationProofSigned")
            .expect("AggregationProofSigned should be present");
        let plaintext_aggregated_index = completion_events
            .iter()
            .position(|event| *event == "PlaintextAggregated")
            .expect("PlaintextAggregated should be present");

        assert_eq!(
            completion_events.len(),
            7 + (2 * c6_fold_steps),
            "Unexpected active aggregator C6/C7 completion event count"
        );
        assert_eq!(
            &completion_events[..2],
            ["ComputeRequest", "ComputeRequest"]
        );
        assert_eq!(
            count_projected_events(completion_events, "ComputeRequest"),
            2 + c6_fold_steps
        );
        assert_eq!(
            count_projected_events(completion_events, "ComputeResponse"),
            2 + c6_fold_steps
        );
        assert_eq!(
            count_projected_events(completion_events, "AggregationProofPending"),
            1
        );
        assert_eq!(
            count_projected_events(completion_events, "AggregationProofSigned"),
            1
        );
        assert_eq!(
            count_projected_events(completion_events, "PlaintextAggregated"),
            1
        );
        assert!(
            aggregation_pending_index < aggregation_signed_index,
            "AggregationProofPending must precede AggregationProofSigned"
        );
        assert_eq!(
            plaintext_aggregated_index,
            completion_events.len() - 1,
            "PlaintextAggregated must be the last active aggregator completion event"
        );
    } else {
        assert_eq!(
            completion_events,
            [
                "ComputeRequest",
                "ComputeResponse",
                "AggregationProofPending",
                "ComputeRequest",
                "ComputeResponse",
                "AggregationProofSigned",
                "PlaintextAggregated",
            ],
            "Unexpected active aggregator plaintext completion flow"
        );
    }

    report.push((
        "Ciphertext published -> PlaintextAggregated",
        publishing_ct_timer.elapsed(),
    ));

    let (plaintext, aggregation_proofs) = h
        .iter()
        .rev()
        .find_map(|e| {
            if let EnclaveEventData::PlaintextAggregated(PlaintextAggregated {
                decrypted_output,
                aggregation_proofs,
                ..
            }) = e.get_data()
            {
                Some((decrypted_output.clone(), aggregation_proofs.clone()))
            } else {
                None
            }
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Expected PlaintextAggregated in events, got: {:?}",
                h.event_types()
            )
        })?;

    assert!(
        !aggregation_proofs.is_empty(),
        "C7 proofs should be present in PlaintextAggregated"
    );

    let results = plaintext
        .into_iter()
        .map(|a| decode_bytes_to_vec_u64(&a.extract_bytes()).expect("error decoding bytes"))
        .collect::<Vec<Vec<u64>>>();

    let results: Vec<u64> = results
        .into_iter()
        .map(|r| r.first().unwrap().clone())
        .collect();

    // Show summation result (mod plaintext modulus)
    let plaintext_modulus = params_raw.clone().plaintext();
    let mut expected_result = vec![0u64; 3];
    for vals in &numbers {
        for j in 0..num_votes_per_voter {
            expected_result[j] = (expected_result[j] + vals[j]) % plaintext_modulus;
        }
    }

    for (i, (res, exp)) in results.iter().zip(expected_result.iter()).enumerate() {
        println!("Tally {i} result = {res} / {exp}");
        assert_eq!(res, exp);
    }

    let mt_report = multithread_report.send(ToReport).await.unwrap();
    println!("{}", mt_report);

    report.push(("Entire Test", whole_test.elapsed()));
    println!("{}", report.serialize());

    Ok(())
}

// ============================================================================
// Networking and P2P Tests
// ============================================================================

#[actix::test]
async fn test_p2p_actor_forwards_events_to_network() -> Result<()> {
    use e3_events::{CiphernodeSelected, EnclaveEvent, TakeEvents, Unsequenced};
    use e3_net::events::GossipData;
    use e3_net::{events::NetEvent, NetEventTranslator};
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tokio::sync::{broadcast, Mutex};

    // Setup elements in test
    let (cmd_tx, mut cmd_rx) = mpsc::channel(100); // Transmit byte events to the network
    let (event_tx, _) = broadcast::channel(100); // Receive byte events from the network
    let system = EventSystem::new();
    let bus = system.handle()?.enable("test");
    let history_collector = bus.history();
    let event_rx = Arc::new(event_tx.subscribe());
    // Pas cmd and event channels to NetEventTranslator
    NetEventTranslator::setup(&bus, &cmd_tx, &event_rx, "my-topic");

    // Capture messages from output on msgs vec
    let msgs: Arc<Mutex<Vec<EnclaveEventData>>> = Arc::new(Mutex::new(Vec::new()));

    let msgs_loop = msgs.clone();

    tokio::spawn(async move {
        // Pull events from command channel
        while let Some(cmd) = cmd_rx.recv().await {
            // If the command is a GossipPublish then extract it and save it whilst sending it to
            // the event bus as if it was gossiped from the network and ended up as an external
            // message this simulates a rebroadcast message
            if let Some(msg) = match cmd {
                e3_net::events::NetCommand::GossipPublish { data, .. } => Some(data),
                _ => None,
            } {
                if let GossipData::GossipBytes(_) = msg {
                    let event: EnclaveEvent<Unsequenced> = msg.clone().try_into().unwrap();
                    let (data, _) = event.split();
                    msgs_loop.lock().await.push(data);
                    event_tx.send(NetEvent::GossipData(msg)).unwrap();
                }
            }
            // if this  manages to broadcast an event to the
            // event bus we will expect to see an extra event on
            // the bus but we don't because we handle this
        }
        anyhow::Ok(())
    });

    let evt_1 = PlaintextAggregated {
        e3_id: E3id::new("1235", 1),
        decrypted_output: vec![ArcBytes::from_bytes(&[1, 2, 3, 4])],
        aggregation_proofs: vec![],
        c6_aggregated_proof: None,
    };

    let evt_2 = PlaintextAggregated {
        e3_id: E3id::new("1236", 1),
        decrypted_output: vec![ArcBytes::from_bytes(&[1, 2, 3, 4])],
        aggregation_proofs: vec![],
        c6_aggregated_proof: None,
    };

    let local_evt_3 = CiphernodeSelected {
        e3_id: E3id::new("1235", 1),
        threshold_m: 2,
        threshold_n: 5,
        ..CiphernodeSelected::default()
    };

    bus.publish_without_context(evt_1.clone())?;
    bus.publish_without_context(evt_2.clone())?;
    bus.publish_without_context(local_evt_3.clone())?; // This is a local event which should not be broadcast to the network

    // check the history of the event bus
    let history = history_collector
        .send(TakeEvents::<EnclaveEvent>::new(3))
        .await?;

    assert_eq!(
        *msgs.lock().await,
        vec![evt_1.clone().into(), evt_2.clone().into()], // notice no local events
        "NetEventTranslator did not transmit correct events to the network"
    );

    assert_eq!(
        history
            .events
            .into_iter()
            .map(|e| e.into_data())
            .collect::<Vec<_>>(),
        vec![evt_1.into(), evt_2.into(), local_evt_3.into()], // all local events that have been broadcast but no
        // events from the loopback
        "NetEventTranslator must not retransmit forwarded event to event bus"
    );

    Ok(())
}

#[actix::test]
async fn test_p2p_actor_forwards_events_to_bus() -> Result<()> {
    let seed = Seed(ChaCha20Rng::seed_from_u64(123).get_seed());

    // Setup elements in test
    let (cmd_tx, _) = mpsc::channel(100); // Transmit byte events to the network
    let (event_tx, event_rx) = broadcast::channel(100); // Receive byte events from the network
    let system = EventSystem::new().with_fresh_bus();
    let bus = system.handle()?.enable("test");
    let history_collector = bus.history();

    NetEventTranslator::setup(&bus, &cmd_tx, &Arc::new(event_rx), "mytopic");

    // Capture messages from output on msgs vec
    let event = E3Requested {
        e3_id: E3id::new("1235", 1),
        threshold_m: 3,
        threshold_n: 3,
        seed: seed.clone(),
        params: ArcBytes::from_bytes(&[1, 2, 3, 4]),
        ..E3Requested::default()
    };

    // lets send an event from the network
    let _ = event_tx.send(NetEvent::GossipData(GossipData::GossipBytes(
        bus.event_from(event.clone(), None)?.to_bytes()?,
    )));

    // check the history of the event bus
    let history = history_collector
        .send(TakeEvents::<EnclaveEvent>::new(1))
        .await?;

    assert_eq!(
        history
            .events
            .into_iter()
            .map(|e| e.into_data())
            .collect::<Vec<EnclaveEventData>>(),
        vec![event.into()]
    );

    Ok(())
}

// ============================================================================
// Legacy Tests Pending Port to trBFV
// ============================================================================

/// Test that stopped keyshares retain their state after restart.
/// This test needs to be ported to the new trBFV system once Sync is completed.
// XXX: ENABLE THIS!!
#[actix::test]
#[ignore = "Needs to be ported to trBFV system after Sync is completed"]
async fn test_stopped_keyshares_retain_state() -> Result<()> {
    use e3_bfv_client::{decode_bytes_to_vec_u64, decode_plaintext_to_vec_u64};
    use e3_data::{GetDump, InMemStore};
    use e3_events::{EventBus, EventBusConfig, GetEvents, Shutdown, TakeEvents};
    use e3_test_helpers::{create_random_eth_addrs, get_common_setup, simulate_libp2p_net};
    use fhe::{
        bfv::PublicKey,
        mbfv::{AggregateIter, PublicKeyShare},
    };
    use fhe_traits::Serialize;
    use std::time::Duration;
    use tokio::time::sleep;

    async fn setup_local_ciphernode(
        bus: &BusHandle,
        rng: &e3_utils::SharedRng,
        logging: bool,
        addr: &str,
        store: Option<actix::Addr<InMemStore>>,
        cipher: &Arc<Cipher>,
        zk_backend: ZkBackend,
    ) -> Result<e3_ciphernode_builder::CiphernodeHandle> {
        let mut builder = CiphernodeBuilder::new(rng.clone(), cipher.clone())
            .with_trbfv()
            .with_zkproof(zk_backend)
            .testmode_with_signer(PrivateKeySigner::random())
            .testmode_with_forked_bus(bus.event_bus())
            .testmode_with_history()
            .testmode_with_errors()
            .with_pubkey_aggregation()
            .with_threshold_plaintext_aggregation()
            .with_sortition_score();

        if let Some(ref in_mem_store) = store {
            builder = builder.with_in_mem_datastore(in_mem_store);
        }

        if logging {
            builder = builder.with_logging()
        }

        let node = builder.build().await?;
        Ok(node)
    }

    async fn create_local_ciphernodes(
        bus: &BusHandle,
        rng: &e3_utils::SharedRng,
        count: u32,
        cipher: &Arc<Cipher>,
        zk_backend: ZkBackend,
    ) -> Result<Vec<e3_ciphernode_builder::CiphernodeHandle>> {
        let eth_addrs = create_random_eth_addrs(count);
        let mut result = vec![];
        for addr in &eth_addrs {
            println!("Setting up eth addr: {}", addr);
            let tuple =
                setup_local_ciphernode(&bus, &rng, true, addr, None, cipher, zk_backend.clone())
                    .await?;
            result.push(tuple);
        }
        simulate_libp2p_net(&result).await;
        Ok(result)
    }

    let (zk_backend, _zk_temp) = setup_test_zk_backend().await;

    let e3_id = E3id::new("1234", 1);
    let (rng, cn1_address, cn1_data, cn2_address, cn2_data, cipher, history, params, crpoly) = {
        let (bus, rng, seed, params, crpoly, _, _) = get_common_setup(None)?;
        let cipher = Arc::new(Cipher::from_password("Don't tell anyone my secret").await?);
        let ciphernodes =
            create_local_ciphernodes(&bus, &rng, 2, &cipher, zk_backend.clone()).await?;
        let eth_addrs = ciphernodes.iter().map(|n| n.address()).collect::<Vec<_>>();

        setup_score_sortition_environment(&bus, &eth_addrs, 1).await?;

        let [cn1, cn2] = &ciphernodes.as_slice() else {
            panic!("Not enough elements")
        };

        // Send e3request
        bus.publish_without_context(E3Requested {
            e3_id: e3_id.clone(),
            threshold_m: 2,
            threshold_n: 2,
            seed: seed.clone(),
            params: ArcBytes::from_bytes(&encode_bfv_params(&params)),
            ..E3Requested::default()
        })?;

        bus.publish_without_context(CommitteeFinalized {
            e3_id: e3_id.clone(),
            committee: eth_addrs.clone(),
            scores: compute_committee_scores(&eth_addrs, &e3_id, seed.clone()),
            chain_id: 1,
        })?;

        let history_collector = cn1.history().unwrap();
        let error_collector = cn1.errors().unwrap();
        let history = history_collector
            .send(TakeEvents::<e3_events::EnclaveEvent>::new(14))
            .await?;
        let errors = error_collector.send(GetEvents::new()).await?;

        assert_eq!(errors.len(), 0);

        // SEND SHUTDOWN!
        bus.publish_without_context(Shutdown)?;

        // This is probably overkill but required to ensure that all the data is written
        sleep(Duration::from_secs(1)).await;

        // Unwrap does not matter as we are in a test
        let cn1_dump = cn1.in_mem_store().unwrap().send(GetDump).await??;
        let cn2_dump = cn2.in_mem_store().unwrap().send(GetDump).await??;

        (
            rng,
            cn1.address(),
            cn1_dump,
            cn2.address(),
            cn2_dump,
            cipher,
            history,
            params,
            crpoly,
        )
    };

    let bus = EventSystem::in_mem()
        .with_event_bus(
            EventBus::<e3_events::EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start(),
        )
        .handle()?
        .enable("cn2");
    let cn1 = setup_local_ciphernode(
        &bus,
        &rng,
        true,
        &cn1_address,
        Some(InMemStore::from_dump(cn1_data, true)?.start()),
        &cipher,
        zk_backend.clone(),
    )
    .await?;
    let cn2 = setup_local_ciphernode(
        &bus,
        &rng,
        true,
        &cn2_address,
        Some(InMemStore::from_dump(cn2_data, true)?.start()),
        &cipher,
        zk_backend.clone(),
    )
    .await?;
    let history_collector = cn1.history().unwrap();
    simulate_libp2p_net(&[cn1, cn2]).await;

    println!("getting collector from cn1.6");

    // get the public key from history.
    let pubkey: PublicKey = history
        .events
        .iter()
        .filter_map(|evt| match evt.get_data() {
            EnclaveEventData::KeyshareCreated(data) => {
                PublicKeyShare::deserialize(&data.pubkey, &params, crpoly.clone()).ok()
            }
            _ => None,
        })
        .aggregate()?;

    // Publish the ciphertext
    use e3_test_helpers::encrypt_ciphertext;
    let raw_plaintext = vec![vec![4, 5]];
    let (ciphertext, expected) = encrypt_ciphertext(&params, pubkey, raw_plaintext)?;
    bus.publish_without_context(CiphertextOutputPublished {
        ciphertext_output: ciphertext
            .iter()
            .map(|ct| ArcBytes::from_bytes(&ct.to_bytes()))
            .collect(),
        e3_id: e3_id.clone(),
    })?;

    let history = history_collector
        .send(TakeEvents::<e3_events::EnclaveEvent>::new(5))
        .await?;

    let actual = history
        .events
        .into_iter()
        .filter_map(|e| match e.into_data() {
            EnclaveEventData::PlaintextAggregated(data) => Some(data),
            _ => None,
        })
        .collect::<Vec<_>>()
        .first()
        .unwrap()
        .clone();

    assert_eq!(
        actual
            .decrypted_output
            .iter()
            .map(|b| decode_bytes_to_vec_u64(b).unwrap())
            .collect::<Vec<Vec<u64>>>(),
        expected
            .iter()
            .map(|p| decode_plaintext_to_vec_u64(p).unwrap())
            .collect::<Vec<Vec<u64>>>()
    );

    Ok(())
}

/// Test that duplicate E3 IDs work correctly with different chain IDs.
/// This test needs to be ported to use trBFV instead of legacy keyshare.
#[actix::test]
#[ignore = "Needs to be ported to trBFV system"]
async fn test_duplicate_e3_id_with_different_chain_id() -> Result<()> {
    use e3_events::{OrderedSet, PublicKeyAggregated, TakeEvents};
    use e3_test_helpers::{
        create_random_eth_addrs, create_shared_rng_from_u64, get_common_setup, simulate_libp2p_net,
    };
    use fhe::{
        bfv::{BfvParameters, PublicKey, SecretKey},
        mbfv::{AggregateIter, CommonRandomPoly, PublicKeyShare},
    };
    use fhe_traits::Serialize;

    type PkSkShareTuple = (PublicKeyShare, SecretKey, String);

    async fn setup_local_ciphernode(
        bus: &BusHandle,
        rng: &e3_utils::SharedRng,
        logging: bool,
        addr: &str,
        store: Option<actix::Addr<e3_data::InMemStore>>,
        cipher: &Arc<Cipher>,
        zk_backend: ZkBackend,
    ) -> Result<e3_ciphernode_builder::CiphernodeHandle> {
        let mut builder = CiphernodeBuilder::new(rng.clone(), cipher.clone())
            .with_trbfv()
            .with_zkproof(zk_backend)
            .testmode_with_signer(PrivateKeySigner::random())
            .testmode_with_forked_bus(bus.event_bus())
            .testmode_with_history()
            .testmode_with_errors()
            .with_pubkey_aggregation()
            .with_threshold_plaintext_aggregation()
            .with_sortition_score();

        if let Some(ref in_mem_store) = store {
            builder = builder.with_in_mem_datastore(in_mem_store);
        }

        if logging {
            builder = builder.with_logging()
        }

        let node = builder.build().await?;
        Ok(node)
    }

    async fn create_local_ciphernodes(
        bus: &BusHandle,
        rng: &e3_utils::SharedRng,
        count: u32,
        cipher: &Arc<Cipher>,
        zk_backend: ZkBackend,
    ) -> Result<Vec<e3_ciphernode_builder::CiphernodeHandle>> {
        let eth_addrs = create_random_eth_addrs(count);
        let mut result = vec![];
        for addr in &eth_addrs {
            println!("Setting up eth addr: {}", addr);
            let tuple =
                setup_local_ciphernode(&bus, &rng, true, addr, None, cipher, zk_backend.clone())
                    .await?;
            result.push(tuple);
        }
        simulate_libp2p_net(&result).await;
        Ok(result)
    }

    fn generate_pk_share(
        params: &Arc<BfvParameters>,
        crp: &CommonRandomPoly,
        rng: &e3_utils::SharedRng,
        addr: &str,
    ) -> Result<PkSkShareTuple> {
        let sk = SecretKey::random(&params, &mut *rng.lock().unwrap());
        let pk = PublicKeyShare::new(&sk, crp.clone(), &mut *rng.lock().unwrap())?;
        Ok((pk, sk, addr.to_owned()))
    }

    fn generate_pk_shares(
        params: &Arc<BfvParameters>,
        crp: &CommonRandomPoly,
        rng: &e3_utils::SharedRng,
        eth_addrs: &Vec<String>,
    ) -> Result<Vec<PkSkShareTuple>> {
        let mut result = vec![];
        for addr in eth_addrs {
            result.push(generate_pk_share(params, crp, rng, addr)?);
        }
        Ok(result)
    }

    fn aggregate_public_key(shares: &Vec<PkSkShareTuple>) -> Result<PublicKey> {
        Ok(shares
            .clone()
            .into_iter()
            .map(|(pk, _, _)| pk)
            .aggregate()?)
    }

    // Setup
    let (bus, rng, seed, params, crpoly, _, _) = get_common_setup(None)?;
    let cipher = Arc::new(Cipher::from_password("Don't tell anyone my secret").await?);
    let (zk_backend, _zk_temp) = setup_test_zk_backend().await;

    // Setup actual ciphernodes and dispatch add events
    let ciphernodes = create_local_ciphernodes(&bus, &rng, 3, &cipher, zk_backend.clone()).await?;
    let eth_addrs = ciphernodes.iter().map(|tup| tup.address()).collect();

    setup_score_sortition_environment(&bus, &eth_addrs, 1).await?;
    setup_score_sortition_environment(&bus, &eth_addrs, 2).await?;

    // Send the computation requested event
    bus.publish_without_context(E3Requested {
        e3_id: E3id::new("1234", 1),
        threshold_m: 2,
        threshold_n: 5,
        seed: seed.clone(),
        params: ArcBytes::from_bytes(&encode_bfv_params(&params)),
        ..E3Requested::default()
    })?;

    bus.publish_without_context(CommitteeFinalized {
        e3_id: E3id::new("1234", 1),
        committee: eth_addrs.clone(),
        scores: compute_committee_scores(&eth_addrs, &E3id::new("1234", 1), seed.clone()),
        chain_id: 1,
    })?;

    // Generate the test shares and pubkey
    let rng_test = create_shared_rng_from_u64(42);
    let test_pubkey = aggregate_public_key(&generate_pk_shares(
        &params, &crpoly, &rng_test, &eth_addrs,
    )?)?;
    let history_collector = ciphernodes.last().unwrap().history().unwrap();
    let history = history_collector
        .send(TakeEvents::<e3_events::EnclaveEvent>::new(28))
        .await?;

    assert_eq!(
        history.events.last().cloned().unwrap().into_data(),
        PublicKeyAggregated {
            pubkey: ArcBytes::from_bytes(&test_pubkey.to_bytes()),
            e3_id: E3id::new("1234", 1),
            nodes: OrderedSet::from(eth_addrs.clone()),
            pk_aggregation_proof: None,
            dkg_aggregated_proof: None,
        }
        .into()
    );

    // Send the computation requested event
    bus.publish_without_context(E3Requested {
        e3_id: E3id::new("1234", 2),
        threshold_m: 2,
        threshold_n: 5,
        seed: seed.clone(),
        params: ArcBytes::from_bytes(&encode_bfv_params(&params)),
        ..E3Requested::default()
    })?;

    bus.publish_without_context(CommitteeFinalized {
        e3_id: E3id::new("1234", 2),
        committee: eth_addrs.clone(),
        scores: compute_committee_scores(&eth_addrs, &E3id::new("1234", 2), seed.clone()),
        chain_id: 2,
    })?;

    let test_pubkey = aggregate_public_key(&generate_pk_shares(
        &params, &crpoly, &rng_test, &eth_addrs,
    )?)?;

    let history = history_collector
        .send(TakeEvents::<e3_events::EnclaveEvent>::new(8))
        .await?;

    assert_eq!(
        history.events.last().cloned().unwrap().into_data(),
        PublicKeyAggregated {
            pubkey: ArcBytes::from_bytes(&test_pubkey.to_bytes()),
            e3_id: E3id::new("1234", 2),
            nodes: OrderedSet::from(eth_addrs.clone()),
            pk_aggregation_proof: None,
            dkg_aggregated_proof: None,
        }
        .into()
    );

    Ok(())
}
