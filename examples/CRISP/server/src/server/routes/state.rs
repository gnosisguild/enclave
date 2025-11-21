// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::server::{
    app_data::AppData,
    models::{GetRoundRequest, WebhookPayload},
    CONFIG,
};
use actix_web::{web, HttpResponse, Responder};
use alloy::primitives::{Bytes, U256};
use e3_sdk::evm_helpers::contracts::{
    EnclaveContract, EnclaveContractFactory, EnclaveWrite, ReadWrite,
};
use log::{error, info};

pub fn setup_routes(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/state")
            .route("/result", web::post().to(get_round_result))
            .route("/all", web::get().to(get_all_round_results))
            .route("/lite", web::post().to(get_round_state_lite))
            // Do we need protection on this endpoint? technically they would need to send a valid proof for it to
            // be included on chain
            .route("/add-result", web::post().to(handle_program_server_result))
            // Get the token holders hashes for a given round
            .route("/token-holders", web::post().to(get_token_holders_hashes)),
    );
}

/// Webhook callback from program server
///
/// # Arguments
/// * `data` - The request data containing the result from the program server
///
/// # Returns
/// * A JSON response indicating the success of the operation
async fn handle_program_server_result(data: web::Json<WebhookPayload>) -> impl Responder {
    let incoming = data.into_inner();

    info!(
        "Received program server result for E3 ID: {:?}, status: {:?}",
        incoming.e3_id, incoming.status
    );

    // Handle failed computation
    if incoming.status == "failed" {
        let error_msg = incoming
            .error
            .unwrap_or_else(|| "Unknown error".to_string());
        error!(
            "Computation failed for E3 ID: {}. Error: {}",
            incoming.e3_id, error_msg
        );

        // TODO: Update E3 state to indicate computation failed
        // TODO: Handle ciphernode rewards for partial work
        // TODO: Emit on-chain event if needed

        return HttpResponse::Ok().json(format!(
            "Computation failed for E3 ID: {}. Error: {}",
            incoming.e3_id, error_msg
        ));
    }

    // Handle successful computation
    if incoming.status != "completed" {
        error!(
            "Unknown status '{}' for E3 ID: {}",
            incoming.status, incoming.e3_id
        );
        return HttpResponse::BadRequest().json(format!("Unknown status: {}", incoming.status));
    }

    // Validate that we have ciphertext and proof for completed status
    if incoming.ciphertext.is_empty() || incoming.proof.is_empty() {
        error!(
            "Missing ciphertext or proof for completed computation E3 ID: {}",
            incoming.e3_id
        );
        return HttpResponse::BadRequest().json(format!(
            "Missing ciphertext or proof for E3 ID: {}",
            incoming.e3_id
        ));
    }

    // Create the contract
    let contract: EnclaveContract<ReadWrite> = match EnclaveContractFactory::create_write(
        &CONFIG.http_rpc_url,
        &CONFIG.enclave_address,
        &CONFIG.private_key,
    )
    .await
    {
        Ok(contract) => contract,
        Err(e) => {
            error!("Failed to create contract: {:?}", e);
            return HttpResponse::InternalServerError()
                .json(format!("Failed to create contract: {}", e));
        }
    };

    // Try the direct call
    let tx_result = contract
        .publish_ciphertext_output(
            U256::from(incoming.e3_id),
            Bytes::from(incoming.ciphertext.clone()),
            Bytes::from(incoming.proof.clone()),
        )
        .await;

    let pending_tx = match tx_result {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to send transaction: {:?}", e);
            return HttpResponse::InternalServerError()
                .json(format!("Failed to send transaction: {}", e));
        }
    };

    info!(
        "Ciphertext output published successfully for E3 ID: {} with tx: {}",
        incoming.e3_id, pending_tx.transaction_hash
    );

    HttpResponse::Ok().json(format!(
        "Ciphertext output published successfully for E3 ID: {}",
        incoming.e3_id
    ))
}

/// Get the result for a given round
///
/// # Arguments
///
/// * `GetRoundRequest` - The request data containing the round ID
///
/// # Returns
///
async fn get_round_result(
    data: web::Json<GetRoundRequest>,
    store: web::Data<AppData>,
) -> impl Responder {
    let incoming = data.into_inner();
    match store.e3(incoming.round_id).get_web_result_request().await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => {
            error!("Error getting E3 state: {:?}", e);
            HttpResponse::InternalServerError().body("Failed to get E3 state")
        }
    }
}

/// Get all the results for all rounds
///
/// # Returns
///
/// * A JSON response containing the results for all rounds
async fn get_all_round_results(store: web::Data<AppData>) -> impl Responder {
    let round_count = match store.current_round().get_current_round_id().await {
        Ok(count) => count,
        Err(e) => {
            info!("Error retrieving round count: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to retrieve round count");
        }
    };

    let mut states = Vec::new();

    // FIXME: This assumes ids are ordered
    for i in 0..round_count + 1 {
        match store.e3(i).get_web_result_request().await {
            Ok(w) => states.push(w),
            Err(e) => {
                info!("Error retrieving state for round {}: {:?}", i, e);
                continue;
            }
        }
    }

    HttpResponse::Ok().json(states)
}

/// Get the state for a given round
///
/// # Arguments
///
/// * `GetRoundRequest` - The request data containing the round ID
///
/// # Returns
///
async fn get_round_state_lite(
    data: web::Json<GetRoundRequest>,
    store: web::Data<AppData>,
) -> impl Responder {
    let incoming = data.into_inner();
    match store.e3(incoming.round_id).get_e3_state_lite().await {
        Ok(state_lite) => HttpResponse::Ok().json(state_lite),
        Err(_) => HttpResponse::InternalServerError().body("Failed to get E3 state"),
    }
}

/// Get the hashes of token holders for a given round
/// The hash is hash(address, token balance)
/// # Arguments
/// * `GetRoundRequest` - The request data containing the round ID
/// # Returns
/// * A JSON response containing the list of token holder hashes
async fn get_token_holders_hashes(
    data: web::Json<GetRoundRequest>,
    store: web::Data<AppData>,
) -> impl Responder {
    let incoming = data.into_inner();

    match store.e3(incoming.round_id).get_token_holder_hashes().await {
        Ok(hashes) => HttpResponse::Ok().json(hashes),
        Err(e) => {
            error!("Error getting token holders hashes: {:?}", e);
            HttpResponse::InternalServerError().body("Failed to get token holders hashes")
        }
    }
}
