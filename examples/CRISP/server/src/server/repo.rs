// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use super::{
    database::generate_emoji,
    models::{CurrentRound, E3Crisp, E3StateLite, WebResultRequest},
};
use e3_sdk::indexer::{models::E3 as EnclaveE3, DataStore, E3Repository, SharedStore};
use eyre::Result;
use log::info;

pub struct CurrentRoundRepository<S: DataStore> {
    store: SharedStore<S>,
}

impl<S: DataStore> CurrentRoundRepository<S> {
    pub fn new(store: SharedStore<S>) -> Self {
        Self { store }
    }

    pub async fn set_current_round(&mut self, value: CurrentRound) -> Result<()> {
        let key = self.current_round_key();
        self.store
            .insert(&key, &value)
            .await
            .map_err(|_| eyre::eyre!("Could not set current_round for '{key}'"))?;
        Ok(())
    }

    pub async fn get_current_round(&self) -> Result<Option<CurrentRound>> {
        let key = self.current_round_key();
        let round = self
            .store
            .get::<CurrentRound>(&key)
            .await
            .map_err(|_| eyre::eyre!("Could get e3 at '{key}'"))?;
        Ok(round)
    }

    pub async fn get_current_round_id(&self) -> Result<u64> {
        let round = self
            .get_current_round()
            .await?
            .ok_or(eyre::eyre!("No current round has been saved"))?;

        Ok(round.id)
    }

    fn current_round_key(&self) -> String {
        format!("_e3:current_round")
    }
}

pub struct CrispE3Repository<S: DataStore> {
    store: SharedStore<S>,
    e3_id: u64,
}

impl<S: DataStore> CrispE3Repository<S> {
    pub fn new(store: SharedStore<S>, e3_id: u64) -> Self {
        Self { store, e3_id }
    }

    async fn set_crisp(&mut self, value: E3Crisp) -> Result<()> {
        let key = self.crisp_key();
        self.store
            .insert(&key, &value)
            .await
            .map_err(|_| eyre::eyre!("Could not store crisp at '{key}'"))?;
        Ok(())
    }

    async fn get_crisp(&self) -> Result<E3Crisp> {
        let key = self.crisp_key();
        let e3_crisp = self
            .store
            .get::<E3Crisp>(&key)
            .await
            .map_err(|e| eyre::eyre!("Could get crisp at '{key}' due to error: {e}"))?
            .ok_or(eyre::eyre!("No data found at {key}"))?;
        Ok(e3_crisp)
    }

    pub async fn start_round(&mut self) -> Result<()> {
        let mut e3_crisp = self.get_crisp().await?;
        e3_crisp.start_time = chrono::Utc::now().timestamp() as u64;
        e3_crisp.status = "Active".to_string();
        self.set_crisp(e3_crisp).await
    }

    pub async fn insert_ciphertext_input(&mut self, vote: Vec<u8>, index: u64) -> Result<()> {
        let key = self.crisp_key();

        self.store
            .modify(&key, |e3_obj: Option<E3Crisp>| {
                e3_obj.map(|mut e| {
                    e.ciphertext_inputs.push((vote.clone(), index));
                    e
                })
            })
            .await
            .map_err(|_| eyre::eyre!("Could not append ciphertext_input for '{key}'"))?;

        Ok(())
    }

    pub async fn initialize_round(
        &mut self,
        token_address: String,
        balance_threshold: String,
    ) -> Result<()> {
        self.set_crisp(E3Crisp {
            has_voted: vec![],
            start_time: 0u64,
            status: "Requested".to_string(),
            votes_option_1: 0,
            votes_option_2: 0,
            emojis: generate_emoji(),
            token_holder_hashes: vec![],
            token_address,
            balance_threshold,
            ciphertext_inputs: vec![],
        })
        .await
    }

    fn get_e3_repo(&self) -> E3Repository<S> {
        E3Repository::new(self.store.clone(), self.e3_id)
    }

    pub async fn get_e3(&self) -> Result<EnclaveE3> {
        let e3 = self.get_e3_repo().get_e3().await?;
        Ok(e3)
    }

    pub async fn get_vote_count(&self) -> Result<u64> {
        let e3_crisp = self.get_crisp().await?;
        Ok(u64::try_from(e3_crisp.ciphertext_inputs.len())?)
    }

    pub async fn update_status(&mut self, value: &str) -> Result<()> {
        let key = self.crisp_key();

        self.store
            .modify(&key, |e3_obj: Option<E3Crisp>| {
                e3_obj.map(|mut e| {
                    e.status = value.to_string();
                    e
                })
            })
            .await
            .map_err(|_| eyre::eyre!("Could not update status for '{key}'"))?;
        Ok(())
    }

    pub async fn set_votes(&mut self, option_1: u64, option_2: u64) -> Result<()> {
        info!("set_votes(option_1:{} option_2:{})", option_1, option_2);
        let key = self.crisp_key();
        self.store
            .modify(&key, |e3_obj: Option<E3Crisp>| {
                e3_obj.map(|mut e| {
                    e.votes_option_1 = option_1;
                    e.votes_option_2 = option_2;
                    e
                })
            })
            .await
            .map_err(|_| eyre::eyre!("Could not append ciphertext_input for '{key}'"))?;
        Ok(())
    }

    pub async fn get_ciphertext_output(&self) -> Result<Vec<u8>> {
        let e3 = self.get_e3().await?;
        Ok(e3.ciphertext_output)
    }

    pub async fn get_committee_public_key(&self) -> Result<Vec<u8>> {
        let e3 = self.get_e3().await?;
        Ok(e3.committee_public_key)
    }

    pub async fn get_web_result_request(&self) -> Result<WebResultRequest> {
        let e3 = self.get_e3().await?;
        let e3_crisp = self.get_crisp().await?;
        Ok(WebResultRequest {
            round_id: e3.id,
            option_1_tally: e3_crisp.votes_option_1,
            option_2_tally: e3_crisp.votes_option_2,
            total_votes: e3_crisp.votes_option_1 + e3_crisp.votes_option_2,
            option_1_emoji: e3_crisp.emojis[0].clone(),
            option_2_emoji: e3_crisp.emojis[1].clone(),
            end_time: e3.expiration,
        })
    }

    pub async fn get_e3_state_lite(&self) -> Result<E3StateLite> {
        let e3 = self.get_e3().await?;
        let e3_crisp = self.get_crisp().await?;
        Ok(E3StateLite {
            emojis: e3_crisp.emojis,
            expiration: e3.expiration,
            id: self.e3_id,
            status: e3_crisp.status,
            chain_id: e3.chain_id,
            duration: e3.duration,
            vote_count: u64::try_from(e3_crisp.ciphertext_inputs.len())?,
            start_time: e3_crisp.start_time,
            start_block: e3.request_block,
            enclave_address: e3.enclave_address,
            committee_public_key: e3.committee_public_key,
            token_address: e3_crisp.token_address,
            balance_threshold: e3_crisp.balance_threshold,
        })
    }

    pub async fn get_ciphertext_inputs(&self) -> Result<Vec<(Vec<u8>, u64)>> {
        let e3_crisp = self.get_crisp().await?;
        Ok(e3_crisp.ciphertext_inputs)
    }

    pub async fn set_ciphertext_output(&mut self, data: Vec<u8>) -> Result<()> {
        self.get_e3_repo().set_ciphertext_output(data).await?;
        Ok(())
    }

    pub async fn has_voted(&self, address: String) -> Result<bool> {
        let e3_crisp = self.get_crisp().await?;
        Ok(e3_crisp.has_voted.contains(&address))
    }

    pub async fn insert_voter_address(&mut self, address: String) -> Result<()> {
        let key = self.crisp_key();
        self.store
            .modify(&key, |e3_obj: Option<E3Crisp>| {
                e3_obj.map(|mut e| {
                    e.has_voted.push(address.clone());
                    e
                })
            })
            .await
            .map_err(|_| eyre::eyre!("Could not insert address on '{key}'"))?;
        Ok(())
    }

    pub async fn remove_voter_address(&mut self, address: &str) -> Result<()> {
        let key = self.crisp_key();
        self.store
            .modify(&key, |e3_obj: Option<E3Crisp>| {
                e3_obj.map(|mut e| {
                    e.has_voted = e
                        .has_voted
                        .into_iter()
                        .filter(|item| item != address)
                        .collect();
                    e
                })
            })
            .await
            .map_err(|_| eyre::eyre!("Could not remove address {address}"))?;
        Ok(())
    }

    pub async fn is_finished(&self) -> Result<bool> {
        let e3 = self.get_crisp().await?;
        Ok(e3.status == "Finished")
    }

    pub async fn set_token_holder_hashes(&mut self, hashes: Vec<String>) -> Result<()> {
        let key = self.crisp_key();

        self.store
            .modify(&key, |e3_obj: Option<E3Crisp>| {
                e3_obj.map(|mut e| {
                    e.token_holder_hashes = hashes.clone();
                    e
                })
            })
            .await
            .map_err(|_| eyre::eyre!("Could not set token_holder_hashes for '{key}'"))?;

        Ok(())
    }

    pub async fn get_token_holder_hashes(&self) -> Result<Vec<String>> {
        let e3_crisp = self.get_crisp().await?;
        Ok(e3_crisp.token_holder_hashes)
    }

    fn crisp_key(&self) -> String {
        let e3_id = self.e3_id;
        format!("_e3:crisp:{e3_id}")
    }
}
