mod database;
pub mod models;
use chrono::Utc;
use clap::Parser;
use compute_provider::FHEInputs;
use core::error::Error;
use database::SledDB;
use enclave_sdk::evm::contracts::{EnclaveContract, EnclaveRead, EnclaveWrite};
use enclave_sdk::evm::events::{E3Activated, InputPublished};
use enclave_sdk::indexer::models::E3;
use enclave_sdk::indexer::{get_e3, DataStore, EnclaveIndexer, SharedStore};
use eyre::{eyre, OptionExt, Result, WrapErr};
use log::info;
use models::{CurrentRound, E3Crisp};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep_until, Instant};
use voting_host::run_compute;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    ws_url: String,

    #[arg(short, long)]
    contract_address: String,

    // TODO: review security of passing private_key on CLI.
    #[arg(short, long)]
    private_key: String,
}

pub struct CrispE3Repository<S: DataStore> {
    store: SharedStore<S>,
    e3_id: u64,
}

impl<S: DataStore> CrispE3Repository<S> {
    pub fn new(store: SharedStore<S>, e3_id: u64) -> Self {
        Self { store, e3_id }
    }

    pub async fn set_crisp(&mut self, value: E3Crisp) -> Result<()> {
        let key = self.crisp_key();
        self.store
            .insert(&key, &value)
            .await
            .map_err(|_| eyre::eyre!("Could not store crisp at '{key}'"))?;
        Ok(())
    }

    pub async fn get_crisp(&self) -> Result<E3Crisp> {
        let key = self.crisp_key();
        let e3_crisp = self
            .store
            .get::<E3Crisp>(&key)
            .await
            .map_err(|_| eyre::eyre!("Could get crisp at '{key}'"))?
            .ok_or(eyre::eyre!("No data found at {key}"))?;
        Ok(e3_crisp)
    }

    pub async fn get_e3(&self) -> Result<E3> {
        let key = self.e3_key();
        let e3 = self
            .store
            .get::<E3>(&key)
            .await
            .map_err(|_| eyre::eyre!("Could get e3 at '{key}'"))?
            .ok_or(eyre::eyre!("No data found at {key}"))?;

        Ok(e3)
    }

    pub async fn set_current_round(&mut self, value: CurrentRound) -> Result<()> {
        let key = self.current_round_key();
        self.store
            .insert(&key, &value)
            .await
            .map_err(|_| eyre::eyre!("Could not set current_round for '{key}'"))?;
        Ok(())
    }

    pub async fn update_status(&mut self, value: &str) -> Result<()> {
        let key = self.crisp_key();

        self.store
            .modify(&key, |e3_obj: Option<E3Crisp>| {
                e3_obj.map(|mut e: E3Crisp| {
                    e.status = value.to_string();
                    e
                })
            })
            .await
            .map_err(|_| eyre::eyre!("Could not update status for '{key}'"))?;
        Ok(())
    }

    fn crisp_key(&self) -> String {
        let e3_id = self.e3_id;
        format!("e3:crisp:{e3_id}")
    }
    fn e3_key(&self) -> String {
        let e3_id = self.e3_id;
        format!("e3:{e3_id}")
    }
    fn current_round_key(&self) -> String {
        format!("e3:current_round")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let pathdb = std::env::current_dir()?.join("database/program");
    let store = SledDB::new(&pathdb.to_str().ok_or_eyre("Bad path provided")?)?;
    let mut indexer = EnclaveIndexer::new(&cli.ws_url, &cli.contract_address, store).await?;
    let contract =
        Arc::new(EnclaveContract::new(&cli.ws_url, &cli.private_key, &cli.contract_address).await?);
    indexer
        .add_event_handler(move |event: E3Activated, store| {
            let e3_id = event.e3Id.to::<u64>();
            let mut repo = CrispE3Repository::new(store, e3_id);

            let expiration = event.expiration.to::<u64>();
            let start_time = Utc::now().timestamp() as u64;

            info!("Handling E3 request with id {}", e3_id);
            async move {
                let e3_crisp = E3Crisp {
                    has_voted: vec![],
                    start_time,
                    status: "Active".to_string(),
                    vote_count: 0,
                    votes_option_1: 0,
                    votes_option_2: 0,
                };

                repo.set_crisp(e3_crisp).await?;
                repo.set_current_round(CurrentRound { id: e3_id }).await?;

                // Calculate expiration time to sleep until
                let expiration = Instant::now()
                    + (UNIX_EPOCH + Duration::from_secs(expiration))
                        .duration_since(SystemTime::now())
                        .unwrap_or_else(|_| Duration::ZERO);

                sleep_until(expiration).await;

                let e3_crisp = repo.get_crisp().await?;

                let e3 = repo.get_e3().await?;
                repo.update_status("Expired").await?;

                if e3_crisp.vote_count > 0 {
                    let fhe_inputs = FHEInputs {
                        params: e3.e3_params,
                        ciphertexts: e3.ciphertext_inputs,
                    };

                    info!("Starting computation for E3: {}", e3_id);
                    repo.update_status("Computing").await?;

                    // Call Compute Provider in a separate thread
                    let (risc0_output, ciphertext) =
                        tokio::task::spawn_blocking(move || run_compute(fhe_inputs)?).await?;

                    info!("Computation completed for E3: {}", e3_id);
                    info!("RISC0 Output: {:?}", risc0_output);

                    repo.update_status("PublishingCiphertext").await?;

                    let tx = contract
                        .clone()
                        .publish_ciphertext_output(
                            e3_id,
                            ciphertext.into(),
                            risc0_output.seal.into(),
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

    let handler = indexer.start()?;
    handler.await?;
    Ok(())
}
