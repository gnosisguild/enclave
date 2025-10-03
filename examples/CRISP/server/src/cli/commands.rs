// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use chrono::Utc;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::CLI_DB;
use alloy::primitives::{Address, Bytes, U256};
use crisp::config::CONFIG;
use e3_sdk::bfv_helpers::{build_bfv_params_arc, encode_bfv_params, params::SET_2048_1032193_1};
use e3_sdk::evm_helpers::contracts::{EnclaveContract, EnclaveRead, EnclaveWrite};
use fhe_rs::bfv::{BfvParameters, Ciphertext, Encoding, Plaintext, PublicKey, SecretKey};
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
struct PKRequest {
    round_id: u64,
    pk_bytes: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CTRequest {
    round_id: u64,
    ct_bytes: Vec<u8>,
}

pub async fn initialize_crisp_round(
    token_address: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Starting new CRISP round with token address: {}",
        token_address
    );
    let contract = EnclaveContract::new(
        &CONFIG.http_rpc_url,
        &CONFIG.private_key,
        &CONFIG.enclave_address,
    )
    .await?;
    let e3_program: Address = CONFIG.e3_program_address.parse()?;

    info!("Enabling E3 Program...");
    match contract.is_e3_program_enabled(e3_program).await {
        Ok(enabled) => {
            if !enabled {
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

    // Convert the token address from hex string to bytes.
    let token_address: Address = token_address.parse()?;
    let custom_params = Bytes::from(token_address.as_slice().to_vec());

    let filter: Address = CONFIG.naive_registry_filter_address.parse()?;
    let threshold: [u32; 2] = [CONFIG.e3_threshold_min, CONFIG.e3_threshold_max];
    let start_window: [U256; 2] = [
        U256::from(Utc::now().timestamp()),
        U256::from(Utc::now().timestamp() + CONFIG.e3_window_size as i64),
    ];
    let duration: U256 = U256::from(CONFIG.e3_duration);
    let e3_params = Bytes::from(encode_bfv_params(&generate_bfv_parameters()));
    let compute_provider_params = ComputeProviderParams {
        name: CONFIG.e3_compute_provider_name.to_string(),
        parallel: CONFIG.e3_compute_provider_parallel,
        batch_size: CONFIG.e3_compute_provider_batch_size,
    };
    let compute_provider_params_bytes = Bytes::from(serde_json::to_vec(&compute_provider_params)?);

    let res = contract
        .request_e3(
            filter,
            threshold,
            start_window,
            duration,
            e3_program,
            e3_params,
            compute_provider_params_bytes,
            custom_params,
        )
        .await?;
    info!("E3 request sent. TxHash: {:?}", res.transaction_hash);

    Ok(())
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

    let contract = EnclaveContract::new(
        &CONFIG.http_rpc_url,
        &CONFIG.private_key,
        &CONFIG.enclave_address,
    )
    .await?;
    let res = contract
        .publish_plaintext_output(U256::from(input_crisp_id), Bytes::from(votes.to_be_bytes()))
        .await?;
    info!("Vote broadcast. TxHash: {:?}", res.transaction_hash);

    Ok(())
}

fn generate_bfv_parameters() -> Arc<BfvParameters> {
    let (degree, plaintext_modulus, moduli) = SET_2048_1032193_1;
    build_bfv_params_arc(degree, plaintext_modulus, &moduli)
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
