use actix::{Actor, Addr};
use anyhow::Result;
use config::AppConfig;
use data::{DataStore, InMemStore, SledStore};
use enclave_core::EventBus;
use router::{Repositories, RepositoriesFactory};

pub fn setup_datastore(config: &AppConfig, bus: &Addr<EventBus>) -> Result<DataStore> {
    if config.use_in_mem_store() {
        return Ok(DataStore::in_mem());
    }

    Ok(DataStore::persistent(&bus, &config.db_file())?)
}

pub fn get_repositories(config: &AppConfig, bus: &Addr<EventBus>) -> Result<Repositories> {
    let store = setup_datastore(config, &bus)?;
    Ok(store.repositories())
}
