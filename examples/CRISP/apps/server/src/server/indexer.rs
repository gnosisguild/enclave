use crate::server::{
    models::CurrentRound,
    repo::{CrispE3Repository, CurrentRoundRepository},
};
use compute_provider::FHEInputs;
use enclave_sdk::{
    evm::{
        contracts::{EnclaveContract, EnclaveRead, EnclaveWrite},
        events::{
            CiphertextOutputPublished, CommitteePublished, E3Activated, InputPublished,
            PlaintextOutputPublished,
        },
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

pub async fn setup_indexer(
    ws_url: &str,
    contract_address: &str,
    store: impl DataStore,
    private_key: &str,
) -> Result<EnclaveIndexer<impl DataStore>> {
    let mut indexer = EnclaveIndexer::new(ws_url, contract_address, store).await?;

    let contract = Arc::new(EnclaveContract::new(&ws_url, &private_key, &contract_address).await?);

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

                    let (risc0_output, ciphertext) =
                        run_compute(fhe_inputs.params, fhe_inputs.ciphertexts)
                            .await
                            .map_err(|e| eyre::eyre!("Error running compute: {e}"))?;

                    info!("Computation completed for E3: {}", e3_id);
                    info!("RISC0 Output: {:?}", risc0_output);

                    repo.update_status("PublishingCiphertext").await?;

                    let tx = contract
                        .clone()
                        .publish_ciphertext_output(
                            event.e3Id,
                            ciphertext.into(),
                            risc0_output.into(),
                        )
                        .await?;

                    info!(
                        "CiphertextOutputPublished event published with tx: {:?}",
                        tx.transaction_hash
                    );
                } else {
                    info!("E3 has no votes to decrypt. Setting status to Finished.");
                    repo.update_status("Finished").await?;
                }
                info!("E3 request handled successfully.");

                Ok(())
            }
        })
        .await;

    // InputPublished
    indexer
        .add_event_handler(move |event: InputPublished, store| {
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store, e3_id);
            async move {
                repo.insert_ciphertext_input(event.data.to_vec(), event.index.to::<u64>())
                    .await?;
                Ok(())
            }
        })
        .await;

    // CiphertextOutputPublished
    indexer
        .add_event_handler(move |event: CiphertextOutputPublished, store| {
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store, e3_id);
            async move {
                repo.set_ciphertext_output(event.ciphertextOutput.to_vec())
                    .await?;

                repo.update_status("CiphertextPublished").await?;
                Ok(())
            }
        })
        .await;

    // PlaintextOutputPublished
    indexer
        .add_event_handler(move |event: PlaintextOutputPublished, store| {
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store, e3_id);
            async move {
                let decoded: Vec<u64> = bincode::deserialize(&event.plaintextOutput.to_vec())?;
                let total_votes = repo.get_vote_count().await?;
                repo.set_plaintext_output(event.plaintextOutput.to_vec())
                    .await?;
                let option_1 = decoded[0];
                let option_2 = total_votes - option_1;
                repo.set_votes(option_1, option_2).await?;

                repo.update_status("Finished").await?;
                Ok(())
            }
        })
        .await;

    Ok(indexer)
}

pub async fn start_indexer(
    ws_url: &str,
    contract_address: &str,
    store: impl DataStore,
    private_key: &str,
) -> Result<()> {
    let ws_url = ws_url.to_string();
    let contract_address = contract_address.to_string();
    let private_key = private_key.to_string();
    let mut indexer = setup_indexer(&ws_url, &contract_address, store, &private_key).await?;
    // CommitteePublished
    indexer
        .add_event_handler(move |event: CommitteePublished, _| {
            let ws_url = ws_url.clone();
            let contract_address = contract_address.clone();
            let private_key = private_key.clone();
            async move {
                let contract =
                    EnclaveContract::new(&ws_url, &private_key, &contract_address).await?;

                let tx = contract.activate(event.e3Id, event.publicKey).await?;
                info!("E3 activated with tx: {:?}", tx.transaction_hash);
                Ok(())
            }
        })
        .await;

    Ok(())
}
