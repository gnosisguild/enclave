// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::server::token_holders::{get_mock_token_holders, BitqueryClient};
use crate::server::{
    models::{CurrentRound, CustomParams},
    program_server_request::run_compute,
    repo::{CrispE3Repository, CurrentRoundRepository},
    token_holders::{build_tree, compute_token_holder_hashes},
    CONFIG,
};
use alloy::sol_types::{sol_data, SolType};

use alloy_primitives::Address;
use e3_sdk::{
    evm_helpers::{
        contracts::{
            EnclaveContract, EnclaveContractFactory, EnclaveRead, EnclaveWrite, ReadWrite,
        },
        events::{
            CiphertextOutputPublished, CommitteePublished, E3Activated, E3Requested,
            PlaintextOutputPublished,
        },
        listener::EventListener,
    },
    indexer::{DataStore, EnclaveIndexer},
};
use eyre::Context;
use log::info;
use num_bigint::BigUint;
use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep_until, Instant};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub async fn register_e3_requested(
    mut indexer: EnclaveIndexer<impl DataStore>,
) -> Result<EnclaveIndexer<impl DataStore>> {
    // E3Requested
    indexer
        .add_event_handler(move |event: E3Requested, store| {
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store.clone(), e3_id);

            info!("E3Requested: {:?}", event);

            async move {
                // Convert custom params bytes back to token address and balance threshold.

                // Use sol_data types instead of primitives
                type CustomParamsTuple = (sol_data::Address, sol_data::Uint<256>);

                let decoded = <CustomParamsTuple as SolType>::abi_decode(&event.e3.customParams)
                    .with_context(|| "Failed to decode custom params from E3 event")?;

                let custom_params = CustomParams {
                    token_address: decoded.0.to_string(),
                    balance_threshold: decoded.1.to_string(),
                };

                let balance_threshold =
                    BigUint::parse_bytes(custom_params.balance_threshold.as_bytes(), 10)
                        .ok_or_else(|| eyre::eyre!("Invalid balance threshold"))?;
                let token_address: Address = custom_params
                    .token_address
                    .parse()
                    .with_context(|| "Invalid token address")?;

                // save the e3 details
                repo.initialize_round(custom_params.token_address, custom_params.balance_threshold)
                    .await?;

                // Get token holders from Bitquery API or mocked data.
                let token_holders = if matches!(CONFIG.chain_id, 31337 | 1337) {
                    info!(
                        "Using mocked token holders for local network (chain_id: {})",
                        CONFIG.chain_id
                    );

                    get_mock_token_holders()
                } else {
                    info!(
                        "Using Bitquery API for network (chain_id: {})",
                        CONFIG.chain_id
                    );

                    let bitquery_client = BitqueryClient::new(CONFIG.bitquery_api_key.clone());
                    bitquery_client
                        .get_token_holders(
                            token_address,
                            balance_threshold,
                            event.e3.requestBlock.to::<u64>(),
                            CONFIG.chain_id,
                            10000, // TODO: this is fine for now, but we need pagination or chunking strategies
                                   // to retrieve large datasets efficiently.
                        )
                        .await
                        .with_context(|| "Bitquery error")?
                };

                if token_holders.is_empty() {
                    return Err(eyre::eyre!(
                        "No eligible token holders found for token address {}.",
                        token_address
                    )
                    .into());
                }

                // Compute Poseidon hashes for token holder address + balance pairs.
                let token_holder_hashes = compute_token_holder_hashes(&token_holders)
                    .with_context(|| "Failed to compute token holder hashes")?;

                repo.set_token_holder_hashes(token_holder_hashes.clone())
                    .await?;

                let tree =
                    build_tree(token_holder_hashes).with_context(|| "Failed to build tree")?;
                let merkle_root = tree
                    .root()
                    .ok_or_else(|| eyre::eyre!("Failed to get merkle root from tree"))?;

                info!("Merkle root: {}", merkle_root);

                // TODO: Publish merkle root on-chain (inputValidator contract).

                Ok(())
            }
        })
        .await;
    Ok(indexer)
}

pub async fn register_e3_activated(
    mut indexer: EnclaveIndexer<impl DataStore>,
) -> Result<EnclaveIndexer<impl DataStore>> {
    // E3Activated
    indexer
        .add_event_handler(move |event: E3Activated, store| {
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store.clone(), e3_id);
            let mut current_round_repo = CurrentRoundRepository::new(store);
            let expiration = event.expiration.to::<u64>();

            info!("Handling E3 request with id {}", e3_id);
            async move {
                repo.start_round().await?;

                current_round_repo
                    .set_current_round(CurrentRound { id: e3_id })
                    .await?;

                // Calculate expiration time to sleep until
                let expiration = Instant::now()
                    + (UNIX_EPOCH + Duration::from_secs(expiration))
                        .duration_since(SystemTime::now())
                        .unwrap_or_else(|_| Duration::ZERO);

                sleep_until(expiration).await;

                let e3: e3_sdk::indexer::models::E3 = repo.get_e3().await?;
                repo.update_status("Expired").await?;

                if repo.get_vote_count().await? > 0 {
                    info!("Starting computation for E3: {}", e3_id);
                    repo.update_status("Computing").await?;

                    let (id, status) = run_compute(
                        e3_id,
                        e3.e3_params,
                        e3.ciphertext_inputs,
                        format!("{}/state/add-result", CONFIG.enclave_server_url),
                    )
                    .await
                    .map_err(|e| eyre::eyre!("Error sending run compute request: {e}"))?;

                    if id != e3_id {
                        return Err(eyre::eyre!(
                            "Computation request returned unexpected E3 ID: expected {}, got {}",
                            e3_id,
                            id
                        )
                        .into());
                    }

                    if status != "processing" {
                        return Err(eyre::eyre!(
                            "Computation request failed with status: {}",
                            status
                        )
                        .into());
                    }

                    info!("Request Computation for E3: {}", e3_id);

                    repo.update_status("PublishingCiphertext").await?;
                } else {
                    info!("E3 has no votes to decrypt. Setting status to Finished.");
                    repo.update_status("Finished").await?;
                }
                info!("E3 request handled successfully.");

                Ok(())
            }
        })
        .await;
    Ok(indexer)
}

pub async fn register_ciphertext_output_published(
    mut indexer: EnclaveIndexer<impl DataStore>,
) -> Result<EnclaveIndexer<impl DataStore>> {
    // CiphertextOutputPublished
    indexer
        .add_event_handler(move |event: CiphertextOutputPublished, store| {
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store, e3_id);
            async move {
                repo.update_status("CiphertextPublished").await?;
                Ok(())
            }
        })
        .await;
    Ok(indexer)
}

pub async fn register_plaintext_output_published(
    mut indexer: EnclaveIndexer<impl DataStore>,
) -> Result<EnclaveIndexer<impl DataStore>> {
    // PlaintextOutputPublished
    indexer
        .add_event_handler(move |event: PlaintextOutputPublished, store| {
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store, e3_id);
            async move {
                info!("CRISP: handling 'PlaintextOutputPublished'");

                // The plaintextOutput from the event contains the result of the FHE computation.
                // The computation sums the encrypted votes: '0' for Option 1, '1' for Option 2.
                // Thus, the decrypted sum directly represents the number of votes for Option 2.
                // The output is expected to be a Vec<u8> in little endian format of u64s.
                let decoded: Vec<u64> = event
                    .plaintextOutput
                    .chunks_exact(8)
                    .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
                    .collect();

                // decoded[0] is the sum of all encrypted votes (0s and 1s).
                // Since Option 1 votes are encrypted as '0' and Option 2 votes as '1',
                // this sum is equivalent to the count of votes for Option 2.
                let option_2 = decoded[0];

                // Retrieve the total number of votes that were cast and recorded for this round.
                let total_votes = repo.get_vote_count().await?;

                // The number of votes for Option 1 can be derived by subtracting
                // the Option 2 votes (the sum from the FHE output) from the total votes.
                let option_1 = total_votes - option_2;

                info!("Vote Count: {:?}", total_votes);
                info!("Votes Option 1: {:?}", option_1);
                info!("Votes Option 2: {:?}", option_2);

                repo.set_votes(option_1, option_2).await?;
                repo.update_status("Finished").await?;
                Ok(())
            }
        })
        .await;
    Ok(indexer)
}

pub async fn register_committee_published(
    mut listener: EventListener,
    contract: EnclaveContract<ReadWrite>,
) -> Result<EventListener> {
    // CommitteePublished
    listener
        .add_event_handler(move |event: CommitteePublished| {
            let contract = contract.clone();
            async move {
                // We need to do this to ensure this is idempotent.
                // TODO: conserve bandwidth and check for E3AlreadyActivated error instead of
                // making two calls to contract
                let e3 = contract.get_e3(event.e3Id).await?;
                if u64::try_from(e3.expiration)? > 0 {
                    info!("E3 already activated '{}'", event.e3Id);
                    return Ok(());
                }

                // Convert milliseconds to seconds for comparison with block.timestamp
                let start_time_ms = e3.startWindow[0].to::<u64>();
                let start_time_secs = start_time_ms / 1000; // Convert to seconds
                let start_time = UNIX_EPOCH + Duration::from_secs(start_time_secs);

                // Get current time
                let now = SystemTime::now();

                // Calculate wait duration
                let wait_duration = match start_time.duration_since(now) {
                    Ok(duration) => {
                        info!("Need to wait {:?} ({}s) until activation", duration, duration.as_secs());
                        duration
                    }
                    Err(_) => {
                        info!("Activating E3");
                        Duration::ZERO
                    }
                };

                // Sleep until start time
                let start_instant = Instant::now() + wait_duration;
                sleep_until(start_instant).await;

                // If not activated activate
                let tx = contract.activate(event.e3Id, event.publicKey).await?;
                info!("E3 activated with tx: {:?}", tx.transaction_hash);
                Ok(())
            }
        })
        .await;
    Ok(listener)
}

pub async fn start_indexer(
    ws_url: &str,
    contract_address: &str,
    registry_filter_address: &str,
    store: impl DataStore,
    private_key: &str,
) -> Result<()> {
    let readonly_contract = EnclaveContractFactory::create_read(ws_url, contract_address).await?;

    let readwrite_contract =
        EnclaveContractFactory::create_write(ws_url, contract_address, private_key).await?;

    let enclave_contract_listener =
        EventListener::create_contract_listener(ws_url, contract_address).await?;

    // CRISP indexer
    let crisp_indexer =
        EnclaveIndexer::new(enclave_contract_listener, readonly_contract, store).await?;
    let crisp_indexer = register_e3_requested(crisp_indexer).await?;
    let crisp_indexer = register_e3_activated(crisp_indexer).await?;
    let crisp_indexer = register_ciphertext_output_published(crisp_indexer).await?;
    let crisp_indexer = register_plaintext_output_published(crisp_indexer).await?;
    crisp_indexer.start();

    // Registry Listener
    let registry_contract_listener =
        EventListener::create_contract_listener(&ws_url, registry_filter_address).await?;
    let registry_listener =
        register_committee_published(registry_contract_listener, readwrite_contract).await?;
    registry_listener.start();

    Ok(())
}
