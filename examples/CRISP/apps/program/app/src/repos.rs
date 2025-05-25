use enclave_sdk::indexer::{models::E3, DataStore, SharedStore};

use crate::models::{CurrentRound, E3Crisp};

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
            .map_err(|_| eyre::eyre!("Could get crisp at '{key}'"))?
            .ok_or(eyre::eyre!("No data found at {key}"))?;
        Ok(e3_crisp)
    }

    pub async fn initialize_round(&self) -> Result<()> {
        self.set_crisp(E3Crisp {
            has_voted: vec![],
            start_time,
            status: "Active".to_string(),
            votes_option_1: 0,
            votes_option_2: 0,
        })
        .await
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

    pub async fn get_vote_count(&self) -> Result<u64> {
        let key = self.e3_key();
        let e3 = self
            .store
            .get::<E3>(&key)
            .await
            .map_err(|_| eyre::eyre!("Could get e3 at '{key}'"))?
            .ok_or(eyre::eyre!("No data found at {key}"))?;

        Ok(e3.ciphertext_inputs.len())
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
                e3_obj.map(|mut e| {
                    e.status = value.to_string();
                    e
                })
            })
            .await
            .map_err(|_| eyre::eyre!("Could not update status for '{key}'"))?;
        Ok(())
    }

    pub async fn append_ciphertext_input(&mut self, data: Vec<u8>, index: u64) -> Result<()> {
        let key = self.e3_key();
        self.store
            .modify(&key, |e3_obj: Option<E3>| {
                e3_obj.map(|mut e| {
                    e.ciphertext_inputs.push((data, index));
                    e
                })
            })
            .await
            .map_err(|_| eyre::eyre!("Could not append ciphertext_input for '{key}'"))?;

        Ok(())
    }

    pub async fn set_plaintext_output(&mut self, data: Vec<u8>) -> Result<()> {
        let key = self.e3_key();
        self.store
            .modify(&key, |e3_obj: Option<E3>| {
                e3_obj.map(|mut e| {
                    e.plaintext_output = data;
                    e
                })
            })
            .await
            .map_err(|_| eyre::eyre!("Could not append ciphertext_input for '{key}'"))?;
        Ok(())
    }

    pub async fn set_votes(&mut self, option_1: u64, option_2: u64) -> Result<()> {
        let key = self.e3_key();
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

    pub async fn set_ciphertext_output(&mut self, data: Vec<u8>) -> Result<()> {
        let key = self.e3_key();
        self.store
            .modify(&key, |e3_obj: Option<E3>| {
                e3_obj.map(|mut e| {
                    e.ciphertext_output = data;
                    e
                })
            })
            .await
            .map_err(|_| eyre::eyre!("Could not append ciphertext_input for '{key}'"))?;
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
