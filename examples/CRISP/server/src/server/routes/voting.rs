// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::server::{
    app_data::AppData,
    database::SledDB,
    models::{VoteRequest, VoteResponse, VoteResponseStatus},
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
/// * `VoteRequest` - The vote data to be broadcast
///
/// # Returns
///
/// * A JSON response indicating the success or failure of the operation
async fn broadcast_encrypted_vote(
    data: web::Json<VoteRequest>,
    store: web::Data<AppData>,
) -> impl Responder {
    let vote_request = data.into_inner();
    info!(
        "[e3_id={}] Broadcasting encrypted vote",
        vote_request.round_id
    );
    // Validate and update vote status
    let has_voted = match store
        .e3(vote_request.round_id)
        .has_voted(vote_request.address.clone())
        .await
    {
        Ok(voted) => voted,
        Err(e) => {
            error!(
                "[e3_id={}] Database error checking vote status: {:?}",
                vote_request.round_id, e
            );
            return HttpResponse::InternalServerError().json("Internal server error");
        }
    };

    if has_voted {
        info!("[e3_id={}] User has already voted", vote_request.round_id);
        return HttpResponse::Ok().json(VoteResponse {
            status: VoteResponseStatus::UserAlreadyVoted,
            tx_hash: None,
            message: Some("User Has Already Voted".to_string()),
        });
    }

    let mut repo = store.e3(vote_request.round_id);

    if let Err(e) = repo
        .insert_voter_address(vote_request.address.clone())
        .await
    {
        error!(
            "[e3_id={}] Database error inserting voter: {:?}",
            vote_request.round_id, e
        );
        return HttpResponse::InternalServerError().json("Internal server error");
    }

    let e3_id = U256::from(vote_request.round_id);

    // encoded_proof is already encoded in JavaScript, just decode from hex
    let hex_str = vote_request
        .encoded_proof
        .strip_prefix("0x")
        .unwrap_or(&vote_request.encoded_proof);
    let encoded_proof = match hex::decode(hex_str) {
        Ok(decoded) => Bytes::from(decoded),
        Err(e) => {
            error!(
                "[e3_id={}] Failed to decode encoded_proof: {:?}",
                vote_request.round_id, e
            );
            // Rollback voter insertion before returning error
            let _ = match repo.remove_voter_address(&vote_request.address).await {
                Ok(_) => (),
                Err(e) => error!("Error rolling back the vote: {e}"),
            };
            return HttpResponse::BadRequest().json(VoteResponse {
                status: VoteResponseStatus::FailedBroadcast,
                tx_hash: None,
                message: Some("Invalid hex encoded proof".to_string()),
            });
        }
    };

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
            error!(
                "[e3_id={}] Contract creation failed: {:?}",
                vote_request.round_id, e
            );
            // Rollback voter insertion before returning error
            // Rollback voter insertion before returning error
            let _ = match repo.remove_voter_address(&vote_request.address).await {
                Ok(_) => (),
                Err(e) => error!("Error rolling back the vote: {e}"),
            };
            return HttpResponse::InternalServerError().json("Internal server error");
        }
    };

    match contract.publish_input(e3_id, encoded_proof).await {
        Ok(hash) => {
            info!(
                "[e3_id={}] Vote broadcasted successfully",
                vote_request.round_id
            );
            HttpResponse::Ok().json(VoteResponse {
                status: VoteResponseStatus::Success,
                tx_hash: Some(hash.transaction_hash.to_string()),
                message: Some("Vote Successful".to_string()),
            })
        }
        Err(e) => handle_vote_error(e, repo, &vote_request.address).await,
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
