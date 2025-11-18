// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crisp_constants::get_default_paramset;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use e3_sdk::bfv_helpers::BfvParams;
use log::info;
use num_bigint::BigUint;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::approve;
use super::CLI_DB;
use alloy::primitives::{Address, Bytes, U256};
use alloy::providers::{Provider, ProviderBuilder};
use crisp::config::CONFIG;
use e3_sdk::bfv_helpers::{build_bfv_params_from_set_arc, encode_bfv_params};
use e3_sdk::evm_helpers::contracts::{EnclaveContract, EnclaveRead, EnclaveWrite, E3};
use fhe::bfv::{BfvParameters, Ciphertext, Encoding, Plaintext, PublicKey, SecretKey};
use fhe_traits::{
    DeserializeParametrized, FheDecoder, FheDecrypter, FheEncoder, FheEncrypter,
    Serialize as FheSerialize,
};
use rand::thread_rng;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
struct FHEParams {
    params: Vec<u8>,
    pk: Vec<u8>,
    sk: Vec<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ComputeProviderParams {
    name: String,
    parallel: bool,
    batch_size: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct CustomParams {
    token_address: String,
    balance_threshold: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct PKRequest {
    round_id: u64,
    pk_bytes: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CTRequest {
    round_id: u64,
    ct_bytes: Vec<u8>,
}

pub async fn get_current_timestamp() -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let provider = ProviderBuilder::new().connect(&CONFIG.http_rpc_url).await?;
    let block = provider
        .get_block_by_number(alloy::eips::BlockNumberOrTag::Latest)
        .await
        .unwrap()
        .ok_or_else(|| anyhow::anyhow!("Latest block not found"))?;

    Ok(block.header.timestamp)
}

pub async fn initialize_crisp_round(
    token_address: &str,
    balance_threshold: &str,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Starting new CRISP round with token address: {} and balance threshold: {}",
        token_address, balance_threshold
    );

    let contract = EnclaveContract::new(
        &CONFIG.http_rpc_url,
        &CONFIG.private_key,
        &CONFIG.enclave_address,
    )
    .await?;
    let e3_program: Address = CONFIG.e3_program_address.parse()?;

    info!("Enabling E3 Program with address: {}", e3_program);
    match contract.is_e3_program_enabled(e3_program).await {
        Ok(enabled) => {
            info!("Debug - E3 Program enabled status: {}", enabled);
            if !enabled {
                info!("E3 Program not enabled, attempting to enable...");
                match contract.enable_e3_program(e3_program).await {
                    Ok(res) => info!("E3 Program enabled. TxHash: {:?}", res.transaction_hash),
                    Err(e) => info!("Error enabling E3 Program: {:?}", e),
                }
            } else {
                info!("E3 Program already enabled");
            }
        }
        Err(e) => info!("Error checking E3 Program enabled: {:?}", e),
    }

    let token_address: Address = token_address.parse()?;
    let balance_threshold = BigUint::parse_bytes(balance_threshold.as_bytes(), 10)
        .ok_or("Invalid balance threshold")?;

    let custom_params = CustomParams {
        token_address: token_address.to_string(),
        balance_threshold: balance_threshold.to_string(),
    };
    // Serialize the custom parameters to bytes.
    let custom_params_bytes = Bytes::from(serde_json::to_vec(&custom_params)?);

    let threshold: [u32; 2] = [CONFIG.e3_threshold_min, CONFIG.e3_threshold_max];
    let mut current_timestamp = get_current_timestamp().await?;
    let mut start_window: [U256; 2] = [
        U256::from(current_timestamp),
        U256::from(current_timestamp + CONFIG.e3_window_size as u64),
    ];
    let duration: U256 = U256::from(CONFIG.e3_duration);
    let e3_params = Bytes::from(encode_bfv_params(&generate_bfv_parameters()));
    let compute_provider_params = ComputeProviderParams {
        name: CONFIG.e3_compute_provider_name.to_string(),
        parallel: CONFIG.e3_compute_provider_parallel,
        batch_size: CONFIG.e3_compute_provider_batch_size,
    };
    let compute_provider_params_bytes = Bytes::from(serde_json::to_vec(&compute_provider_params)?);

    info!("Debug Before Fee Quote - start_window: {:?}", start_window);
    info!(
        "Debug Before Fee Quote - current timestamp: {:?}",
        current_timestamp
    );
    info!("Getting fee quote...");
    let fee_amount = contract
        .get_e3_quote(
            threshold,
            start_window,
            duration,
            e3_program,
            e3_params.clone(),
            compute_provider_params_bytes.clone(),
        )
        .await?;
    info!("Fee required: {} tokens", fee_amount);

    info!("Approving fee token...");
    approve::approve_token(
        &CONFIG.http_rpc_url,
        &CONFIG.private_key,
        &CONFIG.fee_token_address,
        &CONFIG.enclave_address,
        fee_amount,
    )
    .await?;

    current_timestamp = get_current_timestamp().await?;
    start_window = [
        U256::from(current_timestamp),
        U256::from(current_timestamp + CONFIG.e3_window_size as u64),
    ];

    info!("Requesting E3 on contract: {}", CONFIG.enclave_address);

    info!("Debug - threshold: {:?}", threshold);
    info!("Debug - start_window: {:?}", start_window);
    info!("Debug - current timestamp: {:?}", current_timestamp);
    info!("Debug - duration: {}", duration);
    info!("Debug - e3_program: {}", e3_program);

    info!(
        "Debug - Checking ciphernode registry at: {}",
        CONFIG.ciphernode_registry_address
    );

    let (res, e3_id) = contract
        .request_e3(
            threshold,
            start_window,
            duration,
            e3_program,
            e3_params,
            compute_provider_params_bytes,
            custom_params_bytes,
        )
        .await?;
    info!("E3 request sent. TxHash: {:?}", res.transaction_hash);
    let e3_id_u64 = u64::try_from(e3_id)?;
    info!("E3 ID: {}", e3_id_u64);

    Ok(e3_id_u64)
}

pub async fn check_e3_activated(
    e3_id: u64,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let contract =
        EnclaveContract::read_only(&CONFIG.http_rpc_url, &CONFIG.enclave_address).await?;
    let e3: E3 = contract.get_e3(U256::from(e3_id)).await?;
    Ok(u64::try_from(e3.expiration)? > 0)
}

pub async fn activate_e3_round() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let input_e3_id: u64 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter CRISP round ID.")
        .interact_text()?;

    let params = generate_bfv_parameters();
    let (sk, pk) = generate_keys(&params);
    let contract = EnclaveContract::new(
        &CONFIG.http_rpc_url,
        &CONFIG.private_key,
        &CONFIG.enclave_address,
    )
    .await?;
    let pk_bytes = Bytes::from(pk.to_bytes());
    let e3_id = U256::from(input_e3_id);
    let res = contract.activate(e3_id, pk_bytes).await?;
    info!("E3 activated. TxHash: {:?}", res.transaction_hash);

    let e3_params = FHEParams {
        params: encode_bfv_params(&params),
        pk: pk.to_bytes(),
        sk: sk.coeffs.into_vec(),
    };

    let db = CLI_DB.write().await;
    let key = format!("_e3:{}", input_e3_id);
    db.insert(key, serde_json::to_vec(&e3_params)?)?;
    db.flush()?;
    info!("E3 parameters stored in database.");

    Ok(())
}

pub async fn participate_in_existing_round(
    client: &Client,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let input_crisp_id: u64 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter CRISP round ID.")
        .interact_text()?;

    let url = format!("{}/rounds/public-key", CONFIG.enclave_server_url);
    let resp = client
        .post(&url)
        .json(&PKRequest {
            round_id: input_crisp_id,
            pk_bytes: vec![0],
        })
        .send()
        .await?;

    let pk_res: PKRequest = resp.json().await?;
    let params = generate_bfv_parameters();
    let pk_deserialized = PublicKey::from_bytes(&pk_res.pk_bytes, &params)?;

    let vote_choice = get_user_vote()?;
    if let Some(vote) = vote_choice {
        let ct = encrypt_vote(vote, &pk_deserialized, &params)?;
        let contract = EnclaveContract::new(
            &CONFIG.http_rpc_url,
            &CONFIG.private_key,
            &CONFIG.enclave_address,
        )
        .await?;
        let res = contract
            .publish_input(U256::from(input_crisp_id), Bytes::from(ct.to_bytes()))
            .await?;
        info!("Vote broadcast. TxHash: {:?}", res.transaction_hash);
    }

    Ok(())
}

pub async fn decrypt_and_publish_result(
    client: &Client,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let input_crisp_id: u64 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter CRISP round ID.")
        .interact_text()?;

    let url = format!("{}/rounds/ciphertext", CONFIG.enclave_address);
    let resp = client
        .post(&url)
        .json(&CTRequest {
            round_id: input_crisp_id,
            ct_bytes: vec![0],
        })
        .send()
        .await?;

    let ct_res: CTRequest = resp.json().await?;

    let db = CLI_DB.read().await;
    let params_bytes = db
        .get(format!("_e3:{}", input_crisp_id))?
        .ok_or("Key not found")?;
    let e3_params: FHEParams = serde_json::from_slice(&params_bytes)?;
    let params = generate_bfv_parameters();
    let sk_deserialized = SecretKey::new(e3_params.sk, &params);

    let ct = Ciphertext::from_bytes(&ct_res.ct_bytes, &params)?;
    let pt = sk_deserialized.try_decrypt(&ct)?;
    let votes = Vec::<u64>::try_decode(&pt, Encoding::poly())?[0];
    info!("Vote count: {:?}", votes);

    let proof = Bytes::from(vec![0]);

    let contract = EnclaveContract::new(
        &CONFIG.http_rpc_url,
        &CONFIG.private_key,
        &CONFIG.enclave_address,
    )
    .await?;
    let res = contract
        .publish_plaintext_output(
            U256::from(input_crisp_id),
            Bytes::from(votes.to_be_bytes()),
            proof,
        )
        .await?;
    info!("Vote broadcast. TxHash: {:?}", res.transaction_hash);

    Ok(())
}

fn generate_bfv_parameters() -> Arc<BfvParameters> {
    build_bfv_params_from_set_arc(get_default_paramset())
}

fn generate_keys(params: &Arc<BfvParameters>) -> (SecretKey, PublicKey) {
    let mut rng = thread_rng();
    let sk = SecretKey::random(params, &mut rng);
    let pk = PublicKey::new(&sk, &mut rng);
    (sk, pk)
}

fn get_user_vote() -> Result<Option<u64>, Box<dyn std::error::Error + Send + Sync>> {
    let selections = &["Abstain.", "Vote yes.", "Vote no."];
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Please select your voting option.")
        .default(0)
        .items(&selections[..])
        .interact()?;

    match selection {
        0 => Ok(None),
        1 => Ok(Some(1)),
        2 => Ok(Some(0)),
        _ => Err("Invalid selection".into()),
    }
}

fn encrypt_vote(
    vote: u64,
    public_key: &PublicKey,
    params: &std::sync::Arc<BfvParameters>,
) -> Result<Ciphertext, Box<dyn std::error::Error + Send + Sync>> {
    let pt = Plaintext::try_encode(&[vote], Encoding::poly(), params)?;
    Ok(public_key.try_encrypt(&pt, &mut thread_rng())?)
}
