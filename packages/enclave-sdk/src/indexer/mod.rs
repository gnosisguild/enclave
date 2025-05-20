pub mod models;
use std::collections::HashMap;
use thiserror::Error;

// use alloy::primitives::Address;
use async_trait::async_trait;
use eyre::Result;
use models::E3;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::RwLock;

use crate::evm::{
    events::{E3Activated, InputPublished},
    listener::EventListener,
};

type E3Id = u64;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("E3 not found: {0}")]
    E3NotFound(E3Id),
    #[error("Object not serializable: {0}")]
    Serialization(E3Id),
}

#[async_trait]
pub trait DataStore {
    type Error;
    async fn insert<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<(), Self::Error>;
    async fn get<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Self::Error>;
}

pub struct InMemoryStore {
    data: RwLock<HashMap<String, Vec<u8>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl DataStore for InMemoryStore {
    type Error = eyre::Error;

    async fn insert<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.data
            .write()
            .await
            .insert(key.to_string(), bincode::serialize(value)?);
        Ok(())
    }

    async fn get<T: DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<Option<T>, Self::Error> {
        Ok(self
            .data
            .read()
            .await
            .get(key)
            .map(|bytes| bincode::deserialize(bytes))
            .transpose()?)
    }
}

pub struct EnclaveIndexer<D: DataStore> {
    listener: EventListener,
    store: D,
}

impl<D: DataStore> EnclaveIndexer<D> {
    pub async fn new(ws_url: &str, contract_address: &str, store: D) -> Result<Self> {
        let listener = EventListener::create_contract_listener(ws_url, contract_address).await?;

        Ok(Self { store, listener })
    }

    pub async fn initialize(mut self) -> Result<Self> {
        self.listener
            .add_event_handler::<InputPublished>(|input: &InputPublished| {
                // let e3_id = input.e3Id.to::<u64>();
                // let (mut e3, key) = self.get_e3(e3_id).await?;
                Ok(())
            })
            .await;

        Ok(self)
    }

    pub async fn start(&self) -> Result<()> {
        self.listener.listen().await
    }

    pub async fn get_e3(&self, e3_id: u64) -> Result<(E3, String), IndexerError> {
        let key = format!("e3:{}", e3_id);
        match self
            .store
            .get::<E3>(&key)
            .await
            .map_err(|_| IndexerError::Serialization(e3_id))?
        {
            Some(e3) => Ok((e3, key)),
            None => Err(IndexerError::E3NotFound(e3_id)),
        }
    }
}

// pub async fn handle_input_published(input: InputPublished) -> Result<()> {
//     // info!("Handling VoteCast event...");
//
//     let e3_id = input.e3Id.to::<u64>();
//     let (mut e3, key) = get_e3(e3_id).await?;
//
//     // e3.ciphertext_inputs
//     //     .push((input.data.to_vec(), input.index.to::<u64>()));
//     // e3.vote_count += 1;
//
//     GLOBAL_DB.insert(&key, &e3).await?;
//
//     info!("Saved Input with Hash: {:?}", input.inputHash);
//     Ok(())
// }

// pub async fn handle_e3(e3_activated: E3Activated) -> Result<()> {
//     let e3_id = e3_activated.e3Id.to::<u64>();
//     // info!("Handling E3 request with id {}", e3_id);
//
//     // Fetch E3 from the contract
//     let contract = EnclaveContract::new(
//         &CONFIG.http_rpc_url,
//         &CONFIG.private_key,
//         &CONFIG.enclave_address,
//     )
//     .await?;
//
//     let e3 = contract.get_e3(e3_activated.e3Id).await?;
//     info!("Fetched E3 from the contract.");
//     info!("E3: {:?}", e3);
//
//     let start_time = Utc::now().timestamp() as u64;
//     let expiration = e3_activated.expiration.to::<u64>();
//
//     let e3_obj = E3 {
//         // Identifiers
//         id: e3_id,
//         chain_id: CONFIG.chain_id, // Hardcoded for testing
//         enclave_address: CONFIG.enclave_address.clone(),
//
//         // Status-related
//         status: "Active".to_string(),
//         has_voted: vec![],
//         vote_count: 0,
//         votes_option_1: 0,
//         votes_option_2: 0,
//
//         // Timing-related
//         start_time,
//         block_start: e3.requestBlock.to::<u64>(),
//         duration: e3.duration.to::<u64>(),
//         expiration,
//
//         // Parameters
//         e3_params: e3.e3ProgramParams.to_vec(),
//         committee_public_key: e3_activated.committeePublicKey.to_vec(),
//
//         // Outputs
//         ciphertext_output: vec![],
//         plaintext_output: vec![],
//
//         // Ciphertext Inputs
//         ciphertext_inputs: vec![],
//
//         // Emojis
//         emojis: generate_emoji(),
//     };
//
//     // Save E3 to the database
//     let key = format!("e3:{}", e3_id);
//     GLOBAL_DB.insert(&key, &e3_obj).await?;
//
//     // Set Current Round
//     let current_round = CurrentRound { id: e3_id };
//     GLOBAL_DB.insert("e3:current_round", &current_round).await?;
//
//     let expiration = Instant::now()
//         + (UNIX_EPOCH + Duration::from_secs(expiration))
//             .duration_since(SystemTime::now())
//             .unwrap_or_else(|_| Duration::ZERO);
//
//     info!("Expiration: {:?}", expiration);
//
//     // Sleep till the E3 expires (instantly if in the past)
//     sleep_until(expiration).await;
//
//     // Get All Encrypted Votes
//     let (mut e3, _) = get_e3(e3_id).await.unwrap();
//     update_e3_status(e3_id, "Expired".to_string()).await?;
//
//     if e3.vote_count > 0 {
//         info!("E3 FROM DB");
//         info!("Vote Count: {:?}", e3.vote_count);
//
//         let fhe_inputs = FHEInputs {
//             params: e3.e3_params,
//             ciphertexts: e3.ciphertext_inputs,
//         };
//         info!("Starting computation for E3: {}", e3_id);
//         update_e3_status(e3_id, "Computing".to_string()).await?;
//         // Call Compute Provider in a separate thread
//         let (risc0_output, ciphertext) =
//             tokio::task::spawn_blocking(move || run_compute(fhe_inputs).unwrap())
//                 .await
//                 .unwrap();
//
//         info!("Computation completed for E3: {}", e3_id);
//         info!("RISC0 Output: {:?}", risc0_output);
//         update_e3_status(e3_id, "PublishingCiphertext".to_string()).await?;
//         // Params will be encoded on chain to create the journal
//         let tx = contract
//             .publish_ciphertext_output(
//                 e3_activated.e3Id,
//                 ciphertext.into(),
//                 risc0_output.seal.into(),
//             )
//             .await?;
//
//         info!(
//             "CiphertextOutputPublished event published with tx: {:?}",
//             tx.transaction_hash
//         );
//     } else {
//         info!("E3 has no votes to decrypt. Setting status to Finished.");
//         e3.status = "Finished".to_string();
//
//         GLOBAL_DB.insert(&key, &e3).await?;
//     }
//     info!("E3 request handled successfully.");
//     Ok(())
// }
