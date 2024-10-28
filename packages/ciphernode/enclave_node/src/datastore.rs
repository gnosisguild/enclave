use actix::{Actor, Addr};
use anyhow::Result;
use config::AppConfig;
use data::{DataStore, InMemStore, SledStore};
use enclave_core::EventBus;
use router::{Repositories, RepositoriesFactory};

pub fn setup_datastore(config: &AppConfig, bus: &Addr<EventBus>) -> Result<DataStore> {
    let store: DataStore = if !config.use_in_mem_store() {
        (&SledStore::new(&bus, &config.db_file())?.start()).into()
    } else {
        (&InMemStore::new(true).start()).into()
    };
    Ok(store)
}

pub fn get_repositories(config: &AppConfig, bus: &Addr<EventBus>) -> Result<Repositories> {
    let store = setup_datastore(config, &bus)?;
    Ok(store.repositories())
}
