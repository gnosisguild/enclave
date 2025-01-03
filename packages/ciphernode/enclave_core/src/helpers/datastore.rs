use std::path::PathBuf;

use actix::{Actor, Addr};
use anyhow::Result;
use config::AppConfig;
use data::{DataStore, InMemStore, SledStore};
use data::{Repositories, RepositoriesFactory};
use events::{EnclaveEvent, EventBus};

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

pub fn get_repositories(
    config: &AppConfig,
    bus: &Addr<EventBus<EnclaveEvent>>,
) -> Result<Repositories> {
    let store = setup_datastore(config, &bus)?;
    Ok(store.repositories())
}
