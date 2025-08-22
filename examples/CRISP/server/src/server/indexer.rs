// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::server::{
    models::CurrentRound,
    repo::{CrispE3Repository, CurrentRoundRepository},
};
use e3_compute_provider::FHEInputs;
use e3_sdk::{
    evm_helpers::{
        contracts::{
            EnclaveContract, EnclaveContractFactory, EnclaveRead, EnclaveWrite, ReadWrite,
        },
        events::{
            CiphertextOutputPublished, CommitteePublished, E3Activated, PlaintextOutputPublished,
        },
        listener::EventListener,
    },
    indexer::{DataStore, EnclaveIndexer},
};
use log::info;
use program_client::run_compute;
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep_until, Instant};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub async fn register_e3_activated(
    mut indexer: EnclaveIndexer<impl DataStore>,
    contract: EnclaveContract<ReadWrite>,
) -> Result<EnclaveIndexer<impl DataStore>> {
    let contract = Arc::new(contract);
    // E3Activated
    indexer
        .add_event_handler(move |event: E3Activated, store| {
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store.clone(), e3_id);
            let mut current_round_repo = CurrentRoundRepository::new(store);
            let expiration = event.expiration.to::<u64>();
            let contract = contract.clone();
            info!("Handling E3 request with id {}", e3_id);
            async move {
                repo.initialize_round().await?;

                current_round_repo
                    .set_current_round(CurrentRound { id: e3_id })
                    .await?;

                // Calculate expiration time to sleep until
                let expiration = Instant::now()
                    + (UNIX_EPOCH + Duration::from_secs(expiration))
                        .duration_since(SystemTime::now())
                        .unwrap_or_else(|_| Duration::ZERO);

                sleep_until(expiration).await;

                let e3 = repo.get_e3().await?;
                repo.update_status("Expired").await?;

                if repo.get_vote_count().await? > 0 {
                    let fhe_inputs = FHEInputs {
                        params: e3.e3_params,
                        ciphertexts: e3.ciphertext_inputs,
                    };

                    info!("Starting computation for E3: {}", e3_id);
                    repo.update_status("Computing").await?;

                    // call /run_compute 
                    // 0.0.0.0:13151 
                    // pub e3_id: Option<u64>,
                    // #[serde(deserialize_with = "deserialize_hex_string")]
                    // pub params: Vec<u8>,
                    // #[serde(deserialize_with = "deserialize_hex_tuple")]
                    // pub ciphertext_inputs: Vec<(Vec<u8>, u64)>,
                    // pub callback_url: Option<String>,

                    // @todo store the callback url on a const somewhere
                    let (id, status) =
                        run_compute(e3_id, fhe_inputs.params, fhe_inputs.ciphertexts, "127.0.0.1:4000/state/result".to_string())
                            .await
                            .map_err(|e| eyre::eyre!("Error sending run compute request: {e}"))?;

                    if id != e3_id {
                        return Err(eyre::eyre!("Computation request returned unexpected E3 ID: expected {}, got {}", e3_id, id).into());
                    }

                    if status != "processing" {
                        return Err(eyre::eyre!("Computation request failed with status: {}", status).into());
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
    let crisp_indexer = register_e3_activated(crisp_indexer, readwrite_contract.clone()).await?;
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
