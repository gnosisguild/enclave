// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::path::PathBuf;

use actix::Actor;
use anyhow::Result;
use e3_ciphernode_builder::get_enclave_bus_handle;
use e3_config::AppConfig;
use e3_data::{DataStore, InMemStore, SledDb, SledStore};
use e3_data::{Repositories, RepositoriesFactory};
use e3_events::BusHandle;

pub fn get_sled_store(bus: &BusHandle, db_file: &PathBuf) -> Result<DataStore> {
    Ok((&SledStore::new(bus, db_file)?).into())
}

pub fn get_in_mem_store() -> DataStore {
    (&InMemStore::new(true).start()).into()
}

pub fn setup_datastore(config: &AppConfig, bus: &BusHandle) -> Result<DataStore> {
    let store: DataStore = if !config.use_in_mem_store() {
        get_sled_store(&bus, &config.db_file())?
    } else {
        get_in_mem_store()
    };
    Ok(store)
}

pub fn get_repositories(config: &AppConfig) -> Result<Repositories> {
    let bus = get_enclave_bus_handle();
    let store = setup_datastore(config, &bus)?;
    Ok(store.repositories())
}

pub fn close_all_connections() {
    SledDb::close_all_connections();
}
