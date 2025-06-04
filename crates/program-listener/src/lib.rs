use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use e3_compute_provider::FHEInputs;
use e3_config::{chain_config::ChainConfig, AppConfig, ContractAddresses};
use e3_evm_helpers::events::E3Activated;
use e3_indexer::{E3Repository, EnclaveIndexer, InMemoryStore};
use e3_program_client::run_compute;
use e3_support_scripts::ctl_run;
use jsonrpsee::{
    core::client::ClientT,
    http_client::{HttpClient, HttpClientBuilder},
    rpc_params,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep_until, Instant};

fn to_hex(bytes: Vec<u8>) -> String {
    format!("0x{}", hex::encode(bytes))
}

#[derive(Clone)]
struct RpcServer {
    client: HttpClient,
    capabilities: Vec<String>,
}

impl RpcServer {
    pub async fn create(url: &str) -> Result<Self> {
        let client = HttpClientBuilder::default().build(url)?;
        let capabilities = client.request("capabilities", rpc_params![]).await?;
        Ok(Self {
            client,
            capabilities,
        })
    }

    fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability.to_string())
    }

    async fn should_compute(&self) -> Result<bool> {
        let result = self.client.request("shouldCompute", rpc_params![]).await?;
        Ok(result)
    }

    async fn process_output(&self, e3_id: u64, proof: Vec<u8>, ciphertext: Vec<u8>) -> Result<()> {
        let proof = to_hex(proof);
        let ciphertext = to_hex(ciphertext);

        let _: u8 = self
            .client
            .request("processOutput", rpc_params![e3_id, proof, ciphertext])
            .await?;

        Ok(())
    }
}

pub async fn execute(config: &AppConfig, chain_name: &str, json_rpc_server: &str) -> Result<()> {
    let json_rpc_server = json_rpc_server.to_owned();
    let Some(chain) = config.chains().iter().find(|c| c.name == chain_name) else {
        anyhow::bail!("No chain '{chain_name}' found in config.");
    };

    tokio::spawn(async {
        match ctl_run().await {
            Ok(_) => (),
            Err(err) => println!("Error running risc0 compute {err}"),
        }
    });

    let rpc = RpcServer::create(&json_rpc_server).await?;

    if !rpc.has_capability("processOutput") {
        bail!("The JSON_RPC server at {json_rpc_server} must support the `processOutput` method.");
    }

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
        .add_event_handler({
            move |event: E3Activated, store| {
                let rpc = rpc.clone();

                async move {
                    let e3_id = event.e3Id.to::<u64>();
                    let repo = E3Repository::new(store, e3_id);

                    let expiration = event.expiration.to::<u64>();

                    // Calculate expiration time to sleep until
                    let expiration = Instant::now()
                        + (UNIX_EPOCH + Duration::from_secs(expiration))
                            .duration_since(SystemTime::now())
                            .unwrap_or_else(|_| Duration::ZERO);

                    // TODO: save this to the db and poll in a loop instead of use this async future as if
                    // the server crashes we loose state sync
                    sleep_until(expiration).await;

                    let e3 = repo.get_e3().await?;

                    if rpc.has_capability("shouldCompute") {
                        // Asks the JSON rpc server if this is ok to compute
                        if !rpc.should_compute().await.map_err(|e| eyre::eyre!("{e}"))? {
                            return Ok(());
                        }
                    }

                    let fhe_inputs = FHEInputs {
                        params: e3.e3_params,
                        ciphertexts: e3.ciphertext_inputs,
                    };

                    let (proof, ciphertext) =
                        run_compute(fhe_inputs.params, fhe_inputs.ciphertexts)
                            .await
                            .map_err(|e| eyre::eyre!("Error running compute: {e}"))?;

                    rpc.process_output(e3_id, proof, ciphertext)
                        .await
                        .map_err(|e| eyre::eyre!("{e}"))?;

                    Ok(())
                }
            }
        })
        .await;

    Ok(())
}
