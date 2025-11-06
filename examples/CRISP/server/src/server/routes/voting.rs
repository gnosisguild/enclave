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
use alloy::{
    dyn_abi::DynSolValue,
    primitives::{Address, Bytes, U256},
};
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
    error!("[e3_id={}] Broadcasting encrypted vote", vote.round_id);
    // Validate and update vote status
    let has_voted = match store
        .e3(vote.round_id)
        .has_voted(vote.address.clone())
        .await
    {
        Ok(voted) => voted,
        Err(e) => {
            error!(
                "[e3_id={}] Database error checking vote status: {:?}",
                vote.round_id, e
            );
            return HttpResponse::InternalServerError().json("Internal server error");
        }
    };

    if has_voted {
        info!("[e3_id={}] User has already voted", vote.round_id);
        return HttpResponse::Ok().json(VoteResponse {
            status: VoteResponseStatus::UserAlreadyVoted,
            tx_hash: None,
            message: Some("User Has Already Voted".to_string()),
        });
    }

    let mut repo = store.e3(vote.round_id);

    if let Err(e) = repo.insert_voter_address(vote.address.clone()).await {
        error!(
            "[e3_id={}] Database error inserting voter: {:?}",
            vote.round_id, e
        );
        return HttpResponse::InternalServerError().json("Internal server error");
    }

    // Prepare vote data for blockchain
    let public_inputs_array = if vote.public_inputs.is_empty() {
        DynSolValue::Array(vec![])
    } else {
        DynSolValue::Array(
            vote.public_inputs
                .into_iter()
                .map(|pi_array_u8| DynSolValue::FixedBytes(pi_array_u8.into(), 32))
                .collect(),
        )
    };

    let address: Address = vote.address.parse().expect("Invalid address");

    let e3_id = U256::from(vote.round_id);
    let params_value = DynSolValue::Tuple(vec![
        DynSolValue::Bytes(vote.proof),
        public_inputs_array,
        DynSolValue::Bytes(vote.enc_vote_bytes),
        DynSolValue::Address(address),
    ]);

    let encoded_params = Bytes::from(params_value.abi_encode_params());

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
                "[e3_id={}] Database error checking vote status: {:?}",
                vote.round_id, e
            );
            return HttpResponse::InternalServerError().json("Internal server error");
        }
    };

    match contract.publish_input(e3_id, encoded_params).await {
        Ok(hash) => {
            info!("[e3_id={}] Vote broadcasted successfully", vote.round_id);
            HttpResponse::Ok().json(VoteResponse {
                status: VoteResponseStatus::Success,
                tx_hash: Some(hash.transaction_hash.to_string()),
                message: Some("Vote Successful".to_string()),
            })
        }
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
