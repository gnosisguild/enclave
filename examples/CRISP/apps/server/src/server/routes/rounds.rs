// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::server::app_data::AppData;
use crate::server::config::CONFIG;
use crate::server::models::{
    CTRequest, ComputeProviderParams, CronRequestE3, JsonResponse, PKRequest,
};
use actix_web::{web, HttpResponse, Responder};
use alloy::primitives::{Address, Bytes, U256};
use chrono::Utc;
use e3_sdk::bfv_helpers::{build_bfv_params_arc, encode_bfv_params, params::SET_2048_1032193_1};
use e3_sdk::evm_helpers::contracts::{EnclaveContract, EnclaveRead, EnclaveWrite};
use log::{error, info};

pub fn setup_routes(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/rounds")
            .route("/current", web::get().to(get_current_round))
            .route("/public-key", web::post().to(get_public_key))
            .route("/ciphertext", web::post().to(get_ciphertext))
            .route("/request", web::post().to(request_new_round)),
    );
}

/// Request a new E3 round
///
/// # Arguments
///
/// * `data` - The request data containing the cron API key
///
/// # Returns
///
/// * A JSON response indicating the success of the operation
async fn request_new_round(data: web::Json<CronRequestE3>) -> impl Responder {
    if data.cron_api_key != CONFIG.cron_api_key {
        return HttpResponse::Unauthorized().json(JsonResponse {
            response: "Invalid API key".to_string(),
        });
    }

    match initialize_crisp_round().await {
        Ok(_) => HttpResponse::Ok().json(JsonResponse {
            response: "New E3 round requested successfully".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(JsonResponse {
            response: format!("Failed to request new E3 round: {}", e),
        }),
    }
}

/// Get the current E3 round
///
/// # Returns
///
/// * A JSON response containing the current round
async fn get_current_round(store: web::Data<AppData>) -> impl Responder {
    match store.current_round().get_current_round().await {
        Ok(Some(current_round)) => HttpResponse::Ok().json(current_round),
        Ok(None) => HttpResponse::NotFound().json(JsonResponse {
            response: "No current round found".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(JsonResponse {
            response: format!("Failed to retrieve current round: {}", e),
        }),
    }
}

/// Get the ciphertext for a given round
///
/// # Arguments
///
/// * `CTRequest` - The request data containing the round ID
///
/// # Returns
///
/// * A JSON response containing the ciphertext
async fn get_ciphertext(data: web::Json<CTRequest>, store: web::Data<AppData>) -> impl Responder {
    let mut incoming = data.into_inner();

    match store.e3(incoming.round_id).get_ciphertext_output().await {
        Ok(ct_bytes) => {
            incoming.ct_bytes = ct_bytes;
            HttpResponse::Ok().json(incoming)
        }
        Err(e) => HttpResponse::InternalServerError().json(JsonResponse {
            response: format!("Failed to retrieve ciphertext output: {}", e),
        }),
    }
}

/// Get the public key for a given round
///
/// # Arguments
///
/// * `PKRequest` - The request data containing the round ID
///
/// # Returns
///
/// * A JSON response containing the public key
async fn get_public_key(data: web::Json<PKRequest>, store: web::Data<AppData>) -> impl Responder {
    let mut incoming = data.into_inner();

    match store.e3(incoming.round_id).get_committee_public_key().await {
        Ok(pk_bytes) => {
            incoming.pk_bytes = pk_bytes;
            HttpResponse::Ok().json(incoming)
        }
        Err(e) => HttpResponse::InternalServerError().json(JsonResponse {
            response: format!("Failed to retrieve public key: {}", e),
        }),
    }
}

/// Initialize a new CRISP round
///
/// Creates a new CRISP round by enabling the E3 program, generating the necessary parameters, and requesting E3.
///
/// # Returns
///
/// * A result indicating the success of the operation
pub async fn initialize_crisp_round() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting new CRISP round!");
    let contract = EnclaveContract::new(
        &CONFIG.http_rpc_url,
        &CONFIG.private_key,
        &CONFIG.enclave_address,
    )
    .await?;
    let e3_program: Address = CONFIG.e3_program_address.parse()?;

    // Enable E3 Program
    info!("Enabling E3 Program...");
    match contract.is_e3_program_enabled(e3_program).await {
        Ok(enabled) => {
            if !enabled {
                match contract.enable_e3_program(e3_program).await {
                    Ok(res) => println!("E3 Program enabled. TxHash: {:?}", res.transaction_hash),
                    Err(e) => println!("Error enabling E3 Program: {:?}", e),
                }
            } else {
                info!("E3 Program already enabled");
            }
        }
        Err(e) => error!("Error checking E3 Program enabled: {:?}", e),
    }

    info!("Generating parameters...");
    let (degree, plaintext_modulus, moduli) = SET_2048_1032193_1;
    let params = encode_bfv_params(&build_bfv_params_arc(degree, plaintext_modulus, &moduli));

    info!("Requesting E3...");
    let filter: Address = CONFIG.naive_registry_filter_address.parse()?;
    let threshold: [u32; 2] = [CONFIG.e3_threshold_min, CONFIG.e3_threshold_max];
    let start_window: [U256; 2] = [
        U256::from(Utc::now().timestamp()),
        U256::from(Utc::now().timestamp() + CONFIG.e3_window_size as i64),
    ];
    let duration: U256 = U256::from(CONFIG.e3_duration);
    let e3_params = Bytes::from(params);
    let compute_provider_params = ComputeProviderParams {
        name: CONFIG.e3_compute_provider_name.clone(),
        parallel: CONFIG.e3_compute_provider_parallel,
        batch_size: CONFIG.e3_compute_provider_batch_size,
    };
    let compute_provider_params = Bytes::from(bincode::serialize(&compute_provider_params)?);
    let res = contract
        .request_e3(
            filter,
            threshold,
            start_window,
            duration,
            e3_program,
            e3_params,
            compute_provider_params,
        )
        .await?;
    info!("E3 request sent. TxHash: {:?}", res.transaction_hash);

    Ok(())
}
