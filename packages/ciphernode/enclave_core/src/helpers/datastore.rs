use std::path::PathBuf;

use actix::{Actor, Addr};
use anyhow::Result;
use e3_config::AppConfig;
use e3_data::{DataStore, InMemStore, SledDb, SledStore};
use e3_data::{Repositories, RepositoriesFactory};
use events::{get_enclave_event_bus, EnclaveEvent, EventBus};

pub fn get_sled_store(bus: &Addr<EventBus<EnclaveEvent>>, db_file: &PathBuf) -> Result<DataStore> {
    Ok((&SledStore::new(bus, db_file)?).into())
}

pub fn get_in_mem_store() -> DataStore {
    (&InMemStore::new(true).start()).into()
}

pub fn setup_datastore(
    config: &AppConfig,
    bus: &Addr<EventBus<EnclaveEvent>>,
) -> Result<DataStore> {
    let store: DataStore = if !config.use_in_mem_store() {
        get_sled_store(&bus, &config.db_file())?
    } else {
        get_in_mem_store()
    };
    Ok(store)
}

pub fn get_repositories(config: &AppConfig) -> Result<Repositories> {
    let bus = get_enclave_event_bus();
    let store = setup_datastore(config, &bus)?;
    Ok(store.repositories())
}

pub fn close_all_connections() {
    SledDb::close_all_connections();
}
