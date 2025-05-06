use config::{Config as ConfigManager, ConfigError};
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub enclave_server_url: String,
    pub enclave_address: String,
    pub naive_registry_filter_address: String,
    pub e3_program_address: String,
    pub e3_threshold_min: u32,
    pub e3_threshold_max: u32,
    pub e3_window_size: u64,
    pub e3_duration: u64,
    pub e3_compute_provider_name: String,
    pub e3_compute_provider_parallel: bool,
    pub e3_compute_provider_batch_size: u32,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenv().ok();
        ConfigManager::builder()
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()
    }
}

pub static CONFIG: Lazy<Config> =
    Lazy::new(|| Config::from_env().expect("Failed to load configuration"));
