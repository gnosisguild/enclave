use anyhow::{anyhow, Context};
use e3_config::{chain_config::ChainConfig, AppConfig, ContractAddresses};
use e3_indexer::{EnclaveIndexer, InMemoryStore};
pub async fn execute(config: &AppConfig, chain_name: &str) -> anyhow::Result<()> {
    let Some(chain) = config.chains().iter().find(|c| c.name == chain_name) else {
        anyhow::bail!("No chain '{chain_name}' found in config.");
    };
    let ChainConfig {
        rpc_url,
        contracts:
            ContractAddresses {
                enclave: contract_address,
                ..
            },
        ..
    } = chain;

    let indexer = EnclaveIndexer::<InMemoryStore>::from_endpoint_address_in_mem(
        &rpc_url,
        &contract_address.address(),
    )
    .await
    .map_err(|e| anyhow!(e))?;
    Ok(())
}
