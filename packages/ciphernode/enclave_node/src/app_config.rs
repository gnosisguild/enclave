use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ContractAddresses {
    pub enclave: String,
    pub ciphernode_registry: String,
    pub filter_registry: String,
}

#[derive(Debug, Deserialize)]
pub struct ChainConfig {
    pub enabled: Option<bool>,
    pub name: String,
    pub rpc_url: String, // We may need multiple per chain for redundancy at a later point
    pub contracts: ContractAddresses,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub chains: Vec<ChainConfig>,
}
