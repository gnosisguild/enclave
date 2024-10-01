#![crate_name = "database"]
#![crate_type = "lib"]
#![warn(missing_docs, unused_imports)]

use std::{env, sync::Arc, fs, str};
use std::fs::File;
use std::io::Read;
use sled::Db;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct State {
    pub id: String,
    pub nodes: Vec<String>,
    pub e3Ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Computation {
    pub computation_id: String,
    pub committee: Vec<(String, String)>,
    pub pk_share: Vec<u8>,
    pub pk_shares: Vec<Vec<u8>>,
    pub rng_seed: u64,
}

#[derive(Clone)]
pub struct EnclaveDB {
    pub db: Db,
    pub db_path: String,
}

impl EnclaveDB {
    pub fn new() -> Self {
        let path = env::current_dir().unwrap();
        let mut pathst = path.clone().display().to_string();
        pathst.push_str("/config.json");
        let mut file = File::open(pathst.clone()).unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();

        let mut pathdbst = path.display().to_string();
        let mut num = rand::thread_rng().gen_range(0..1000);
        
        pathdbst.push_str("/database/ciphernode-");
        pathdbst.push_str(&num.to_string());
        println!("Node database path {:?}", pathdbst);
        let db = sled::open(pathdbst.clone()).unwrap();
        Self { db, db_path: pathdbst }
    }

    pub fn get(&mut self, key: &str) -> Vec<u8> {
        let mut pathdbst = self.db_path.clone();
        pathdbst.push_str("/");
        pathdbst.push_str(key);
        let mut result;
        if self.db.get(pathdbst.clone()).unwrap() == None {
            result = vec![];
        } else {
            result = self.db.get(pathdbst.clone()).unwrap().unwrap().to_vec();
        }
        result
    }

    pub fn get_state(&mut self, key: &str) -> State {
        let mut pathdbst = self.db_path.clone();
        pathdbst.push_str("/");
        pathdbst.push_str(key);
        let state_out = self.db.get(pathdbst.clone()).unwrap().unwrap();
        let state_out_str = str::from_utf8(&state_out).unwrap();
        let state_out_struct: State = serde_json::from_str(&state_out_str).unwrap();
        state_out_struct
    }

    pub fn get_computation_state(&mut self, key: &str) -> Computation {
        let mut pathdbst = self.db_path.clone();
        pathdbst.push_str("/");
        pathdbst.push_str(key);
        let state_out = self.db.get(pathdbst.clone()).unwrap().unwrap();
        let state_out_str = str::from_utf8(&state_out).unwrap();
        let state_out_struct: Computation = serde_json::from_str(&state_out_str).unwrap();
        state_out_struct
    }

    pub fn insert(&mut self, key: &str, data: Vec<u8>) {
        let mut pathdbst = self.db_path.clone();
        pathdbst.push_str("/");
        pathdbst.push_str(key);
        self.db.insert(pathdbst.clone(), data).unwrap();
    }

    pub fn insert_state(&mut self, key: &str, state: State) {
        let mut pathdbst = self.db_path.clone();
        pathdbst.push_str("/");
        pathdbst.push_str(key);
        let state_str = serde_json::to_string(&state).unwrap();
        let state_bytes = state_str.into_bytes();
        self.db.insert(pathdbst.clone(), state_bytes).unwrap();
    }

    pub fn insert_computation_state(&mut self, key: &str, state: Computation) {
        let mut pathdbst = self.db_path.clone();
        pathdbst.push_str("/");
        pathdbst.push_str(key);
        let state_str = serde_json::to_string(&state).unwrap();
        let state_bytes = state_str.into_bytes();
        self.db.insert(pathdbst.clone(), state_bytes).unwrap();
    }
}