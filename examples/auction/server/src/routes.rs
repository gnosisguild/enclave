use actix_web::{web, HttpResponse, Responder};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use fhe::bfv::Ciphertext;
use fhe_traits::{DeserializeParametrized, FheEncrypter, Serialize};
use rand::rngs::OsRng;
use serde::Deserialize;
use serde_json::json;

use auction_example::{self, BID_BITS};

use crate::auction::{Auction, AuctionResult, AuctionState, Bid};
use crate::AppState;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct BidRequest {
    pub address: String,
    pub ciphertext: String, // base64-encoded ciphertext bytes
}

#[derive(Deserialize)]
pub struct EncryptRequest {
    pub bid: u64,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn health() -> impl Responder {
    HttpResponse::Ok().json(json!({ "status": "ok" }))
}

pub async fn create_auction(state: web::Data<AppState>) -> impl Responder {
    let id = state
        .next_id
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let auction = Auction::new(id);

    state.auctions.lock().unwrap().insert(id, auction);

    let pk_b64 = B64.encode(&state.keys.pk_bytes);
    HttpResponse::Ok().json(json!({
        "id": id,
        "public_key": pk_b64,
    }))
}

pub async fn get_auction(state: web::Data<AppState>, path: web::Path<u64>) -> impl Responder {
    let id = path.into_inner();
    let auctions = state.auctions.lock().unwrap();

    match auctions.get(&id) {
        Some(auction) => {
            let pk_b64 = B64.encode(&state.keys.pk_bytes);
            HttpResponse::Ok().json(json!({
                "id": auction.id,
                "state": auction.state,
                "num_bids": auction.bids.len(),
                "public_key": pk_b64,
                "result": auction.result,
            }))
        }
        None => HttpResponse::NotFound().json(json!({ "error": "auction not found" })),
    }
}

pub async fn submit_bid(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<BidRequest>,
) -> impl Responder {
    let id = path.into_inner();
    let mut auctions = state.auctions.lock().unwrap();

    let auction = match auctions.get_mut(&id) {
        Some(a) => a,
        None => return HttpResponse::NotFound().json(json!({ "error": "auction not found" })),
    };

    if auction.state != AuctionState::Open {
        return HttpResponse::BadRequest().json(json!({ "error": "auction is not open" }));
    }

    let ct_bytes = match B64.decode(&body.ciphertext) {
        Ok(b) => b,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({ "error": "invalid base64 ciphertext" }))
        }
    };

    // Validate that the ciphertext deserializes
    if Ciphertext::from_bytes(&ct_bytes, &state.keys.params).is_err() {
        return HttpResponse::BadRequest()
            .json(json!({ "error": "invalid ciphertext for current parameters" }));
    }

    auction.bids.push(Bid {
        address: body.address.clone(),
        ciphertext_bytes: ct_bytes,
    });

    log::info!(
        "Auction {}: bid from '{}' (#{} total)",
        id,
        body.address,
        auction.bids.len()
    );

    HttpResponse::Ok().json(json!({
        "ok": true,
        "num_bids": auction.bids.len(),
    }))
}

pub async fn encrypt_bid_handler(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<EncryptRequest>,
) -> impl Responder {
    let id = path.into_inner();

    // Check auction exists
    {
        let auctions = state.auctions.lock().unwrap();
        if !auctions.contains_key(&id) {
            return HttpResponse::NotFound().json(json!({ "error": "auction not found" }));
        }
    }

    if body.bid >= (1 << BID_BITS) {
        return HttpResponse::BadRequest().json(json!({
            "error": format!("bid must be less than {}", 1u64 << BID_BITS)
        }));
    }

    let pt = auction_example::encode_bid(body.bid, &state.keys.params);
    let ct = state
        .keys
        .pk
        .try_encrypt(&pt, &mut OsRng)
        .expect("encryption failed");
    let ct_b64 = B64.encode(ct.to_bytes());

    HttpResponse::Ok().json(json!({ "ciphertext": ct_b64 }))
}

pub async fn close_auction(state: web::Data<AppState>, path: web::Path<u64>) -> impl Responder {
    let id = path.into_inner();

    // Phase 1: extract bid data, mark as computing
    let bids_data: Vec<(String, Vec<u8>)> = {
        let mut auctions = state.auctions.lock().unwrap();
        let auction = match auctions.get_mut(&id) {
            Some(a) => a,
            None => {
                return HttpResponse::NotFound().json(json!({ "error": "auction not found" }))
            }
        };
        if auction.state != AuctionState::Open {
            return HttpResponse::BadRequest()
                .json(json!({ "error": "auction is not open for closing" }));
        }
        if auction.bids.is_empty() {
            return HttpResponse::BadRequest().json(json!({ "error": "no bids submitted" }));
        }
        auction.state = AuctionState::Computing;
        auction
            .bids
            .iter()
            .map(|b| (b.address.clone(), b.ciphertext_bytes.clone()))
            .collect()
    };

    log::info!("Auction {}: closing with {} bids", id, bids_data.len());

    // Phase 2: deserialize ciphertexts and run tournament
    let ciphertexts: Vec<Ciphertext> = bids_data
        .iter()
        .map(|(_, ct_bytes)| {
            Ciphertext::from_bytes(ct_bytes, &state.keys.params).expect("invalid ciphertext")
        })
        .collect();

    log::info!("Auction {}: running homomorphic comparison tournament...", id);

    let (winner_idx, winning_bid) = auction_example::find_winner(
        &ciphertexts,
        &state.keys.eval_key,
        &state.keys.relin_key,
        &state.keys.sk,
        &state.keys.params,
    );

    let winner_address = bids_data[winner_idx].0.clone();

    log::info!(
        "Auction {}: winner is '{}' with bid {}",
        id,
        winner_address,
        winning_bid
    );

    // Phase 3: store result
    let result = AuctionResult {
        winner_address: winner_address.clone(),
        winning_bid,
    };

    {
        let mut auctions = state.auctions.lock().unwrap();
        let auction = auctions.get_mut(&id).unwrap();
        auction.state = AuctionState::Complete;
        auction.result = Some(result.clone());
    }

    HttpResponse::Ok().json(json!({
        "winner_address": result.winner_address,
        "winning_bid": result.winning_bid,
    }))
}

pub async fn get_result(state: web::Data<AppState>, path: web::Path<u64>) -> impl Responder {
    let id = path.into_inner();
    let auctions = state.auctions.lock().unwrap();

    match auctions.get(&id) {
        Some(auction) => match &auction.result {
            Some(result) => HttpResponse::Ok().json(json!({
                "winner_address": result.winner_address,
                "winning_bid": result.winning_bid,
            })),
            None => HttpResponse::BadRequest()
                .json(json!({ "error": "auction not yet complete", "state": auction.state })),
        },
        None => HttpResponse::NotFound().json(json!({ "error": "auction not found" })),
    }
}
