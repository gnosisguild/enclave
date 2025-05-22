use super::events::InputPublished;
use crate::server::{
    config::CONFIG,
    database::{db_get, db_insert, generate_emoji, get_e3, update_e3_status},
    models::{CurrentRound, E3},
};
use alloy::{
    primitives::{Address, Bytes, FixedBytes, U256},
    providers::Provider,
    rpc::types::{BlockNumberOrTag, Filter, Log},
    sol_types::SolEvent,
};
use compute_provider::FHEInputs;
use enclave_sdk::evm::contracts::{
    EnclaveContract, EnclaveRead, EnclaveReadOnlyProvider, EnclaveWrite, ReadOnly, ReadWrite,
    E3 as ContractE3,
};
use enclave_sdk::indexer::DataStore;
use futures::future::join_all;
use log::{error, info, warn};
use std::{
    collections::HashMap,
    error::Error,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::{sleep_until, Instant};
use voting_host::run_compute;

/// Type alias for results with a boxed error.
type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
pub async fn sync_server() -> Result<()> {
    info!("Starting server synchronization...");

    let contract = Arc::new(
        EnclaveContract::new(
            &CONFIG.http_rpc_url,
            &CONFIG.private_key,
            &CONFIG.enclave_address,
        )
        .await?,
    );

    // Retrieve the current round from the database.
    let current_round = match db_get::<CurrentRound>("e3:current_round").await? {
        Some(round) => round,
        None => {
            info!("No current round found in DB. Exiting sync process. Will compute next round.");
            return Ok(());
        }
    };
    info!("Current round: {}", current_round.id);

    // Fetch the latest E3 from the database and the contract.
    let (latest_db_e3, _) = get_e3(current_round.id).await?;
    let contract_e3_id = contract.get_e3_id().await?.to::<u64>();
    if contract_e3_id == 0 {
        warn!("No E3 IDs found in the contract.");
        return Ok(());
    }
    let latest_contract_e3_id = contract_e3_id - 1;

    // Check if synchronization is needed.
    if latest_db_e3.status == "Finished" && latest_db_e3.id == latest_contract_e3_id {
        info!("Database is up to date with the contract. No sync needed.");
        return Ok(());
    }

    // Identify the last finished E3 in the database.
    let last_finished_e3_id = find_last_finished_e3_id(latest_db_e3.id).await?;
    info!("Last finished E3 ID: {:?}", last_finished_e3_id);

    // Determine the range of E3 IDs to synchronize.
    let start_sync_id = last_finished_e3_id.map_or(0, |id| id + 1);
    let sync_ids: Vec<u64> = (start_sync_id..=latest_contract_e3_id).collect();
    info!("Syncing E3s: {:?}", sync_ids);

    // Determine the starting block for fetching events.
    let from_block = contract
        .get_e3(U256::from(start_sync_id))
        .await?
        .requestBlock
        .to::<u64>();
    info!("From block: {}", from_block);

    // Fetch relevant events from the blockchain.
    let events = Arc::new(fetch_events(contract.clone(), from_block).await?);

    // Synchronize each E3 concurrently.
    join_all(sync_ids.into_iter().map(|e3_id| {
        let contract = contract.clone();
        let events = events.clone();
        async move {
            if let Err(e) = sync_e3(U256::from(e3_id), contract, events).await {
                error!("Failed to sync E3 {}: {:?}", e3_id, e);
            }
        }
    }))
    .await;

    // Update the current round in the database.
    let new_current_round = CurrentRound {
        id: latest_contract_e3_id,
    };
    GLOBAL_DB
        .insert("e3:current_round", &new_current_round)
        .await?;

    info!("Server synchronization completed.");
    Ok(())
}

/// Finds the last finished E3 ID in the database.
async fn find_last_finished_e3_id(latest_db_id: u64) -> Result<Option<u64>> {
    for id in (0..=latest_db_id).rev() {
        let (e3, _) = match get_e3(id).await {
            Ok(e3) => e3,
            Err(_) => continue,
        };
        if e3.status == "Finished" {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// Fetches events from the blockchain starting from a specific block.
async fn fetch_events(
    contract: Arc<EnclaveContract<ReadWrite>>,
    from_block: u64,
) -> Result<HashMap<U256, Vec<Log>>> {
    let filter = Filter::new()
        .from_block(BlockNumberOrTag::Number(from_block))
        .to_block(BlockNumberOrTag::Latest)
        .address(Address::from_str(&CONFIG.enclave_address)?)
        .event(InputPublished::SIGNATURE);

    let logs = contract.provider.get_logs(&filter).await.map_err(|e| {
        error!("Error fetching logs: {:?}", e);
        e
    })?;

    let mut events_by_e3_id = HashMap::new();
    for log in logs {
        let input = log.log_decode::<InputPublished>()?.data().clone();
        events_by_e3_id
            .entry(input.e3Id)
            .or_insert_with(Vec::new)
            .push(log);
    }

    Ok(events_by_e3_id)
}

/// Synchronizes a single E3.
async fn sync_e3(
    e3_id: U256,
    contract: Arc<EnclaveContract<ReadWrite>>,
    published_events: Arc<HashMap<U256, Vec<Log>>>,
) -> Result<()> {
    let events_clone = published_events.clone();
    let contract_clone = contract.clone();
    let contract_e3 = contract.get_e3(e3_id).await?;

    // Exit early if the E3 is not yet activated.
    if contract_e3.committeePublicKey == FixedBytes::<32>::default() {
        info!("E3 {} not yet activated", e3_id);
        return Ok(());
    }

    let expiration = calculate_expiration(&contract_e3.expiration)?;
    let now = Instant::now();

    if contract_e3.ciphertextOutput == FixedBytes::<32>::default() {
        if now >= expiration {
            info!("E3 {} expired, computing and publishing ciphertext.", e3_id);
            tokio::spawn(async move {
                if let Err(e) =
                    compute_and_publish_ciphertext(e3_id, contract_clone, events_clone).await
                {
                    error!("Error computing and publishing ciphertext: {:?}", e);
                }
            });
        } else {
            info!("E3 {} still active, waiting until expiration", e3_id);
            sleep_until(expiration).await;
            // After sleeping, re-fetch events
            let events = Arc::new(
                fetch_events(contract.clone(), contract_e3.requestBlock.to::<u64>()).await?,
            );

            tokio::spawn(async move {
                if let Err(e) = compute_and_publish_ciphertext(e3_id, contract_clone, events).await
                {
                    error!("Error computing and publishing ciphertext: {:?}", e);
                }
            });
        }
        return Ok(());
    }

    if contract_e3.plaintextOutput == Bytes::default() {
        info!("E3 {} waiting for plaintext output", e3_id);
        return Ok(());
    }

    // Sync with the database.
    let vote_count = published_events
        .get(&e3_id)
        .map_or(0, |logs| logs.len() as u64);
    sync_e3_with_db(e3_id, &contract_e3, vote_count).await?;

    Ok(())
}

/// Calculates the expiration time based on the contract's expiration field.
fn calculate_expiration(expiration_secs: &U256) -> Result<Instant> {
    let expiration_duration = UNIX_EPOCH + Duration::from_secs(expiration_secs.to::<u64>());
    let duration_since_now = expiration_duration
        .duration_since(SystemTime::now())
        .unwrap_or_else(|_| Duration::ZERO);
    Ok(Instant::now() + duration_since_now)
}

/// Computes and publishes the ciphertext output.
async fn compute_and_publish_ciphertext(
    e3_id: U256,
    contract: Arc<EnclaveContract<ReadWrite>>,
    events: Arc<HashMap<U256, Vec<Log>>>,
) -> Result<()> {
    let ciphertext_inputs = events
        .get(&e3_id)
        .map(|logs| {
            logs.iter()
                .map(|log| {
                    let input = log.log_decode::<InputPublished>().unwrap().data().clone();
                    (input.data.to_vec(), input.index.to::<u64>())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if ciphertext_inputs.is_empty() {
        info!("No ciphertext inputs found for E3 {}", e3_id);
        update_e3_status(e3_id.to::<u64>(), "Finished".to_string()).await?;
        return Ok(());
    }

    // Update vote count
    let mut db_e3 = get_e3(e3_id.to::<u64>()).await?.0;
    db_e3.vote_count = ciphertext_inputs.len() as u64;
    GLOBAL_DB
        .insert(&format!("e3:{}", e3_id.to::<u64>()), &db_e3)
        .await?;

    let contract_e3 = contract.get_e3(e3_id).await?;
    let fhe_inputs = FHEInputs {
        params: contract_e3.e3ProgramParams.to_vec(),
        ciphertexts: ciphertext_inputs,
    };

    let (risc0_output, ciphertext) =
        tokio::task::spawn_blocking(move || run_compute(fhe_inputs).unwrap())
            .await
            .unwrap();

    let tx = contract
        .publish_ciphertext_output(e3_id, ciphertext.into(), risc0_output.seal.into())
        .await?;

    info!(
        "Ciphertext published for round {}. TxHash: {:?}",
        e3_id, tx.transaction_hash
    );

    Ok(())
}

/// Synchronizes the E3 data with the database.
async fn sync_e3_with_db(e3_id: U256, contract_e3: &ContractE3, vote_count: u64) -> Result<()> {
    let (mut db_e3, key) = match get_e3(e3_id.to::<u64>()).await {
        Ok(e3) => e3,
        Err(_) => {
            let new_e3 = E3 {
                id: e3_id.to::<u64>(),
                chain_id: CONFIG.chain_id,
                enclave_address: CONFIG.enclave_address.clone(),
                status: "Finished".to_string(),
                has_voted: vec![],
                vote_count,
                votes_option_1: 0,
                votes_option_2: 0,
                start_time: contract_e3.startWindow[0].to::<u64>(),
                block_start: contract_e3.requestBlock.to::<u64>(),
                duration: contract_e3.duration.to::<u64>(),
                expiration: contract_e3.expiration.to::<u64>(),
                e3_params: contract_e3.e3ProgramParams.to_vec(),
                committee_public_key: contract_e3.committeePublicKey.to_vec(),
                ciphertext_output: contract_e3.ciphertextOutput.to_vec(),
                plaintext_output: contract_e3.plaintextOutput.to_vec(),
                ciphertext_inputs: vec![],
                emojis: generate_emoji(),
            };
            (new_e3, format!("e3:{}", e3_id.to::<u64>()))
        }
    };

    db_e3.plaintext_output = contract_e3.plaintextOutput.to_vec();
    db_e3.status = "Finished".to_string();

    // Decode plaintext output to obtain vote counts.
    let decoded: Vec<u64> = bincode::deserialize(&db_e3.plaintext_output).unwrap_or(vec![0, 0]);

    if decoded.len() >= 2 {
        db_e3.votes_option_2 = decoded[0];
        db_e3.votes_option_1 = decoded[1];
        db_e3.vote_count = db_e3.votes_option_1 + db_e3.votes_option_2;
    } else {
        warn!("Unexpected plaintext output format for E3 {}", e3_id);
    }

    db_insert(&key, &db_e3).await?;
    info!("E3 {} synced with DB", e3_id);

    Ok(())
}
