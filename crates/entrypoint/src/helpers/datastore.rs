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

/// Creates a sled-backed DataStore using the provided bus and database file.
///
/// Constructs a `SledStore` with `bus` and `db_file` and converts it into a `DataStore`.
/// Propagates any construction error from `SledStore`.
///
/// # Examples
///
/// ```no_run
/// let bus: BusHandle = /* obtain a BusHandle */ unimplemented!();
/// let db_file = std::path::PathBuf::from("my_db.sled");
/// let datastore = get_sled_store(&bus, &db_file).expect("failed to create sled store");
/// ```
pub fn get_sled_store(bus: &BusHandle, db_file: &PathBuf) -> Result<DataStore> {
    Ok((&SledStore::new(bus, db_file)?).into())
}

pub fn get_in_mem_store() -> DataStore {
    (&InMemStore::new(true).start()).into()
}

/// Selects and constructs the application's persistent or in-memory data store based on configuration.
///
/// If the configuration indicates use of an on-disk store, initializes a Sled-backed store with the configured
/// database file; otherwise initializes an in-memory store.
///
/// # Returns
///
/// `Ok(DataStore)` containing the initialized store on success, or an `Err` describing any construction failure.
///
/// # Examples
///
/// ```
/// let config = AppConfig::default();
/// let bus = BusHandle::new();
/// let store = setup_datastore(&config, &bus).unwrap();
/// // Use `store`...
/// ```
pub fn setup_datastore(config: &AppConfig, bus: &BusHandle) -> Result<DataStore> {
    let store: DataStore = if !config.use_in_mem_store() {
        get_sled_store(&bus, &config.db_file())?
    } else {
        get_in_mem_store()
    };
    Ok(store)
}

/// Returns a repositories view configured according to the provided application config.
///
/// Obtains an enclave bus handle from the configuration, initializes the appropriate
/// DataStore (sled-backed or in-memory) and returns the `Repositories` view from that store.
///
/// # Parameters
///
/// - `config`: application configuration that determines which datastore to initialize and
///   is used to obtain the enclave bus handle.
///
/// # Returns
///
/// A `Repositories` view backed by the initialized `DataStore`.
///
/// # Examples
///
/// ```no_run
/// let config = AppConfig::default();
/// let repos = get_repositories(&config).expect("failed to get repositories");
/// ```
pub fn get_repositories(config: &AppConfig) -> Result<Repositories> {
    let bus = get_enclave_bus_handle(config)?;
    let store = setup_datastore(config, &bus)?;
    Ok(store.repositories())
}

pub fn close_all_connections() {
    SledDb::close_all_connections();
}