// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::server::{
    app_data::AppData,
    database::SledDB,
    models::{EncryptedVote, VoteResponse, VoteResponseStatus},
    repo::CrispE3Repository,
    CONFIG,
};
use actix_web::{web, HttpResponse, Responder};
use alloy::primitives::{Bytes, U256};
use e3_sdk::evm_helpers::contracts::{EnclaveContract, EnclaveWrite};
use eyre::Error;
use log::{error, info};

pub fn setup_routes(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/voting").route("/broadcast", web::post().to(broadcast_encrypted_vote)),
    );
}

/// Broadcast an encrypted vote to the blockchain
///
/// # Arguments
///
/// * `EncryptedVote` - The vote data to be broadcast
///
/// # Returns
///
/// * A JSON response indicating the success or failure of the operation
async fn broadcast_encrypted_vote(
    data: web::Json<EncryptedVote>,
    store: web::Data<AppData>,
) -> impl Responder {
    let vote = data.into_inner();

    // Validate and update vote status
    let has_voted = match store
        .e3(vote.round_id)
        .has_voted(vote.address.clone())
        .await
    {
        Ok(voted) => voted,
        Err(e) => {
            log::error!("Database error checking vote status: {:?}", e);
            return HttpResponse::InternalServerError().json("Internal server error");
        }
    };

    if has_voted {
        return HttpResponse::Ok().json(VoteResponse {
            status: VoteResponseStatus::UserAlreadyVoted,
            tx_hash: None,
            message: Some("User Has Already Voted".to_string()),
        });
    }

    let mut repo = store.e3(vote.round_id);

    if let Err(e) = repo.insert_voter_address(vote.address.clone()).await {
        log::error!("Database error inserting voter: {:?}", e);
        return HttpResponse::InternalServerError().json("Internal server error");
    }

    let e3_id = U256::from(vote.round_id);

    // Decode hex string to Vec<u8>, then convert to Bytes
    let input_bytes = hex::decode(vote.crisp_inputs.strip_prefix("0x").unwrap_or(&vote.crisp_inputs))
    .map_err(|e| {
        log::error!("Failed to decode hex string: {:?}", e);
        HttpResponse::InternalServerError().json("Invalid hex string")
    }).unwrap();

    let input_bytes_alloy = Bytes::from(input_bytes);

    // Broadcast vote to blockchain
    let contract = match EnclaveContract::new(
        &CONFIG.http_rpc_url,
        &CONFIG.private_key,
        &CONFIG.enclave_address,
    )
    .await
    {
        Ok(c) => c,
        Err(e) => {
            log::error!("Database error checking vote status: {:?}", e);
            return HttpResponse::InternalServerError().json("Internal server error");
        }
    };

    match contract.publish_input(e3_id, input_bytes_alloy).await {
        Ok(hash) => HttpResponse::Ok().json(VoteResponse {
            status: VoteResponseStatus::Success,
            tx_hash: Some(hash.transaction_hash.to_string()),
            message: Some("Vote Successful".to_string()),
        }),
        Err(e) => handle_vote_error(e, repo, &vote.address).await,
    }
}

/// Handle the vote error
///
/// # Arguments
///
/// * `e` - The error that occurred
/// * `state_data` - The state data to be rolled back
/// * `key` - The key for the state data
/// * `address` - The address for the vote
async fn handle_vote_error(
    e: Error,
    mut repo: CrispE3Repository<SledDB>,
    address: &str,
) -> HttpResponse {
    info!("Error while sending vote transaction: {:?}", e);

    // Rollback the vote
    match repo.remove_voter_address(address).await {
        Ok(_) => (),
        Err(err) => error!("Error rolling back the vote: {err}"),
    };

    HttpResponse::Ok().json(VoteResponse {
        status: VoteResponseStatus::FailedBroadcast,
        tx_hash: None,
        message: Some("Failed to broadcast vote".to_string()),
    })
}
