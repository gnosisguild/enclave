// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::server::token_holders::{get_mock_token_holders, EtherscanClient};
use crate::server::{
    models::{CurrentRound, CustomParams},
    program_server_request::run_compute,
    repo::{CrispE3Repository, CurrentRoundRepository},
    token_holders::{build_tree, compute_token_holder_hashes},
    CONFIG,
};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::sol_types::{sol_data, SolType};
use alloy_primitives::{Address, U256};
use e3_sdk::{
    bfv_helpers::decode_bytes_to_vec_u64,
    evm_helpers::{
        contracts::{EnclaveRead, EnclaveWrite, ReadWrite},
        events::{
            CiphertextOutputPublished, CommitteePublished, E3Activated, E3Requested,
            PlaintextOutputPublished,
        },
    },
    indexer::{DataStore, EnclaveIndexer, SharedStore},
};
use evm_helpers::{CRISPContractFactory, InputPublished};
use eyre::Context;
use log::info;
use num_bigint::BigUint;
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub async fn register_e3_requested(
    indexer: EnclaveIndexer<impl DataStore, ReadWrite>,
) -> Result<EnclaveIndexer<impl DataStore, ReadWrite>> {
    // E3Requested
    indexer
        .add_event_handler(move |event: E3Requested, ctx| {
            let store = ctx.store();
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store.clone(), e3_id);

            info!("[e3_id={}] E3Requested: {:?}", e3_id, event);

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

                // Get token holders from Etherscan API or mocked data.
                let token_holders = if matches!(CONFIG.chain_id, 31337 | 1337) {
                    info!(
                        "[e3_id={}] Using mocked token holders for local network (chain_id: {})",
                        e3_id, CONFIG.chain_id
                    );

                    get_mock_token_holders()
                } else {
                    info!(
                        "[e3_id={}] Using Etherscan API for network (chain_id: {})",
                        e3_id, CONFIG.chain_id
                    );

                    let etherscan_client =
                        EtherscanClient::new(CONFIG.etherscan_api_key.clone(), CONFIG.chain_id);
                    etherscan_client
                        .get_token_holders_with_voting_power(
                            token_address,
                            event.e3.requestBlock.to::<u64>(),
                            &CONFIG.http_rpc_url,
                            U256::from_str_radix(&balance_threshold.to_string(), 10).map_err(
                                |e| {
                                    eyre::eyre!(
                                        "[e3_id={}] Failed to convert balance threshold to U256: {}",
                                        e3_id,
                                        e
                                    )
                                },
                            )?,
                        )
                        .await
                        .map_err(|e| eyre::eyre!("Etherscan error: {}", e))?
                };

                if token_holders.is_empty() {
                    return Err(eyre::eyre!(
                        "[e3_id={}] No eligible token holders found for token address {}.",
                        e3_id,
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

                info!("[e3_id={}] Merkle root: {}", e3_id, merkle_root);

                // Convert merkle root from hex string to U256.
                let merkle_root_bytes = hex::decode(&merkle_root)
                    .with_context(|| format!("[e3_id={}] Merkle root is not valid hex", e3_id))?;
                let merkle_root_u256 = U256::from_be_slice(&merkle_root_bytes);

                // Convert balance_threshold from BigUint to U256.
                let balance_threshold_bytes = balance_threshold.to_bytes_be();
                let balance_threshold_u256 = U256::from_be_slice(&balance_threshold_bytes);

                // Convert e3Id from u64 to U256
                let e3_id_u256 = U256::from(e3_id);

                info!(
                    "[e3_id={}] Calling setRoundData with root: {}, token: {}, threshold: {}",
                    e3_id, merkle_root_u256, token_address, balance_threshold_u256
                );

                let contract = CRISPContractFactory::create_write(
                    &CONFIG.http_rpc_url,
                    &CONFIG.e3_program_address,
                    &CONFIG.private_key,
                )
                .await
                .with_context(|| {
                    format!("[e3_id={}] Failed to create CRISP contract", e3_id)
                })?;

                let receipt = contract
                    .set_round_data(e3_id_u256, merkle_root_u256, token_address, balance_threshold_u256)
                    .await
                    .with_context(|| {
                        format!("[e3_id={}] Failed to call setRoundData", e3_id)
                    })?;

                info!(
                    "[e3_id={}] setRoundData successful. TxHash: {:?}",
                    e3_id, receipt.transaction_hash
                );

                Ok(())
            }
        })
        .await;
    Ok(indexer)
}

pub async fn register_e3_activated(
    indexer: EnclaveIndexer<impl DataStore, ReadWrite>,
) -> Result<EnclaveIndexer<impl DataStore, ReadWrite>> {
    // E3Activated
    indexer
        .add_event_handler(move |event: E3Activated, ctx| {
            let store = ctx.store();
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store.clone(), e3_id);
            let mut current_round_repo = CurrentRoundRepository::new(store);
            let expiration = event.expiration.to::<u64>();

            info!("[e3_id={}] Handling E3 request", e3_id);
            async move {
                repo.start_round().await?;

                current_round_repo
                    .set_current_round(CurrentRound { id: e3_id })
                    .await?;

                info!("[e3_id={}] Registering hook for {}", e3_id, expiration);
                ctx.do_later(expiration, move |_, ctx| {
                    info!("Running....");
                    handle_e3_input_deadline_expiration(e3_id, ctx.store())
                });
                Ok(())
            }
        })
        .await;
    Ok(indexer)
}

async fn handle_e3_input_deadline_expiration(
    e3_id: u64,
    store: SharedStore<impl DataStore>,
) -> eyre::Result<()> {
    let mut repo = CrispE3Repository::new(store.clone(), e3_id);
    let e3: e3_sdk::indexer::models::E3 = repo.get_e3().await?;

    repo.update_status("Expired").await?;

    if repo.get_vote_count().await? > 0 {
        info!("[e3_id={}] Starting computation for E3", e3_id);
        repo.update_status("Computing").await?;

        let votes = repo.get_ciphertext_inputs().await?;

        let (id, status) = run_compute(
            e3_id,
            e3.e3_params,
            votes,
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
            return Err(eyre::eyre!("Computation request failed with status: {}", status).into());
        }

        info!("[e3_id={}] Request Computation for E3", e3_id);

        repo.update_status("PublishingCiphertext").await?;
    } else {
        info!(
            "[e3_id={}] E3 has no votes to decrypt. Setting status to Finished.",
            e3_id
        );
        repo.update_status("Finished").await?;
    }
    info!("[e3_id={}] E3 request handled successfully.", e3_id);

    Ok(())
}

pub async fn register_ciphertext_output_published(
    indexer: EnclaveIndexer<impl DataStore, ReadWrite>,
) -> Result<EnclaveIndexer<impl DataStore, ReadWrite>> {
    // CiphertextOutputPublished
    indexer
        .add_event_handler(move |event: CiphertextOutputPublished, ctx| {
            let store = ctx.store();
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store, e3_id);
            async move {
                info!("[e3_id={}] Handling CiphertextOutputPublished", e3_id);
                repo.update_status("CiphertextPublished").await?;
                Ok(())
            }
        })
        .await;
    Ok(indexer)
}

pub async fn register_plaintext_output_published(
    indexer: EnclaveIndexer<impl DataStore, ReadWrite>,
) -> Result<EnclaveIndexer<impl DataStore, ReadWrite>> {
    // PlaintextOutputPublished
    indexer
        .add_event_handler(move |event: PlaintextOutputPublished, ctx| {
            let store = ctx.store();
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store, e3_id);
            async move {
                info!("[e3_id={}] Handling PlaintextOutputPublished", e3_id);

                // The plaintextOutput from the event contains the result of the FHE computation.
                // The computation sums the encrypted votes: '0' for Option 1, '1' for Option 2.
                // Thus, the decrypted sum directly represents the number of votes for Option 2.
                // The output is expected to be a Vec<u8> in little endian format of u64s.
                let decoded = decode_bytes_to_vec_u64(&event.plaintextOutput)?;

                // decoded[0] is the sum of all encrypted votes (0s and 1s).
                // Since Option 1 votes are encrypted as '0' and Option 2 votes as '1',
                // this sum is equivalent to the count of votes for Option 2.
                let option_2 = decoded[0];

                // Retrieve the total number of votes that were cast and recorded for this round.
                let total_votes = repo.get_vote_count().await?;

                // The number of votes for Option 1 can be derived by subtracting
                // the Option 2 votes (the sum from the FHE output) from the total votes.
                let option_1 = total_votes - option_2;

                info!("[e3_id={}] Vote Count: {:?}", e3_id, total_votes);
                info!("[e3_id={}] Votes Option 1: {:?}", e3_id, option_1);
                info!("[e3_id={}] Votes Option 2: {:?}", e3_id, option_2);

                repo.set_votes(option_1, option_2).await?;
                repo.update_status("Finished").await?;
                Ok(())
            }
        })
        .await;
    Ok(indexer)
}

pub async fn register_committee_published(
    indexer: EnclaveIndexer<impl DataStore, ReadWrite>,
) -> Result<EnclaveIndexer<impl DataStore, ReadWrite>> {
    // CommitteePublished
    indexer
        .add_event_handler(move |event: CommitteePublished, ctx| {
            async move {
                let contract = ctx.contract();
                // We need to do this to ensure this is idempotent.
                // TODO: conserve bandwidth and check for E3AlreadyActivated error instead of
                // making two calls to contract
                let e3 = contract.get_e3(event.e3Id).await?;
                if u64::try_from(e3.expiration)? > 0 {
                    info!("[e3_id={}] E3 already activated", event.e3Id);
                    return Ok(());
                }

                // Read Start time in Seconds
                let start_time = e3.startWindow[0].to::<u64>();
                info!("[e3_id={}] Start time: {}", event.e3Id, start_time);

                // Get current time
                let now = get_current_timestamp_rpc().await?;
                info!("[e3_id={}] Current time: {}", event.e3Id, now);

                //////////////////////////////////////////////////////
                // XXX: FIX ME

                // Calculate wait duration
                let wait_duration = if start_time > now {
                    let secs = start_time - now;
                    info!(
                        "[e3_id={}] Need to wait {} seconds until activation",
                        event.e3Id, secs
                    );
                    Duration::from_secs(secs)
                } else {
                    info!("[e3_id={}] Activating E3", event.e3Id);
                    Duration::ZERO
                };
                info!("[e3_id={}] Wait duration: {:?}", event.e3Id, wait_duration);

                // Sleep until start time
                if !wait_duration.is_zero() {
                    sleep(wait_duration).await;
                }

                ///////////////////////////////////////////////////////

                // If not activated activate
                let tx = contract.activate(event.e3Id, event.publicKey).await?;
                info!(
                    "[e3_id={}] E3 activated with tx: {:?}",
                    event.e3Id, tx.transaction_hash
                );
                Ok(())
            }
        })
        .await;
    Ok(indexer)
}

pub async fn get_current_timestamp_rpc() -> eyre::Result<u64> {
    let provider = ProviderBuilder::new().connect(&CONFIG.http_rpc_url).await?;
    let block = provider
        .get_block_by_number(alloy::eips::BlockNumberOrTag::Latest)
        .await?
        .ok_or_else(|| eyre::eyre!("Latest block not found"))?;

    Ok(block.header.timestamp)
}

pub async fn register_input_published(
    indexer: EnclaveIndexer<impl DataStore, ReadWrite>,
) -> Result<EnclaveIndexer<impl DataStore, ReadWrite>> {
    indexer
        .add_event_handler(move |event: InputPublished, ctx| {
            let e3_id = event.e3Id.to::<u64>();
            let store = ctx.store();
            let mut repo = CrispE3Repository::new(store.clone(), e3_id);
            async move {
                println!(
                    "InputPublished: e3_id={}, index={}, data=0x{}...",
                    event.e3Id,
                    event.index,
                    hex::encode(&event.vote[..8.min(event.vote.len())])
                );

                repo.insert_ciphertext_input(event.vote.to_vec(), event.index.to::<u64>())
                    .await?;
                Ok(())
            }
        })
        .await;
    Ok(indexer)
}

pub async fn start_indexer(
    url: &str,
    contract_address: &str,
    registry_address: &str,
    crisp_address: &str,
    store: SharedStore<impl DataStore>,
    private_key: &str,
) -> Result<()> {
    info!("CRISP: Creating indexer...");
    let crisp_indexer = EnclaveIndexer::new_with_write_contract(
        url,
        &[contract_address, registry_address, crisp_address],
        store,
        private_key,
    )
    .await?;
    info!("CRISP: Indexer registering handlers...");

    let crisp_indexer = register_e3_requested(crisp_indexer).await?;
    let crisp_indexer = register_e3_activated(crisp_indexer).await?;
    let crisp_indexer = register_ciphertext_output_published(crisp_indexer).await?;
    let crisp_indexer = register_plaintext_output_published(crisp_indexer).await?;
    let crisp_indexer = register_committee_published(crisp_indexer).await?;
    let crisp_indexer = register_input_published(crisp_indexer).await?;
    info!("CRISP: Indexer finished registering handlers!");
    crisp_indexer.listen().await?;
    info!("CRISP: Indexer listen loop has finished!");
    Ok(())
}
