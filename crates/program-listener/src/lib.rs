use anyhow::anyhow;
use e3_compute_provider::FHEInputs;
use e3_config::{chain_config::ChainConfig, AppConfig, ContractAddresses};
use e3_evm_helpers::events::E3Activated;
use e3_indexer::{E3Repository, EnclaveIndexer, InMemoryStore};
use e3_program_client::run_compute;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep_until, Instant};

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

    let mut indexer = EnclaveIndexer::<InMemoryStore>::from_endpoint_address_in_mem(
        &rpc_url,
        &contract_address.address(),
    )
    .await
    .map_err(|e| anyhow!(e))?;

    indexer
        .add_event_handler(move |event: E3Activated, store| async move {
            let e3_id = event.e3Id.to::<u64>();
            let repo = E3Repository::new(store, e3_id);

            let expiration = event.expiration.to::<u64>();
            // Calculate expiration time to sleep until
            let expiration = Instant::now()
                + (UNIX_EPOCH + Duration::from_secs(expiration))
                    .duration_since(SystemTime::now())
                    .unwrap_or_else(|_| Duration::ZERO);

            sleep_until(expiration).await;

            let e3 = repo.get_e3().await?;

            // TODO: How can we provide custom conditions for whether or not the calculation should be
            // performed?
            // - pipe to process?
            // - gRPC server and back?
            // - Webhook?

            let fhe_inputs = FHEInputs {
                params: e3.e3_params,
                ciphertexts: e3.ciphertext_inputs,
            };

            let (risc0_output, ciphertext) = run_compute(fhe_inputs.params, fhe_inputs.ciphertexts)
                .await
                .map_err(|e| eyre::eyre!("Error running compute: {e}"))?;

            // TODO: How can we provide custom conditions for whether or not the calculation should be
            // performed?
            // - pipe to process?
            // - gRPC server and back?
            // - Webhook?

            Ok(())
        })
        .await;

    Ok(())
}
