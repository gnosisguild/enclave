use actix_web::{web, HttpResponse, Responder};
use log::info;

use crate::server::database::{get_e3, GLOBAL_DB};
use crate::server::models::{E3StateLite, CurrentRound, GetRoundRequest, WebResultRequest};

pub fn setup_routes(config: &mut web::ServiceConfig) {
    config
        .service(
            web::scope("/state")
                .route("/result", web::post().to(get_round_result))
                .route("/all", web::get().to(get_all_round_results))
                .route("/lite", web::post().to(get_round_state_lite))
        );
}

/// Get the result for a given round
/// 
/// # Arguments
/// 
/// * `GetRoundRequest` - The request data containing the round ID
/// 
/// # Returns
/// 
async fn get_round_result(data: web::Json<GetRoundRequest>) -> impl Responder {
    let incoming = data.into_inner();
    
    match get_e3(incoming.round_id).await {
        Ok((state, _)) => {
            let response: WebResultRequest = state.into();
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            info!("Error getting E3 state: {:?}", e);
            HttpResponse::InternalServerError().body("Failed to get E3 state")
        }
    }
}

/// Get all the results for all rounds
/// 
/// # Returns
/// 
/// * A JSON response containing the results for all rounds
async fn get_all_round_results() -> impl Responder {
    let round_count = match GLOBAL_DB.get::<CurrentRound>("e3:current_round").await {
        Ok(count) => count.unwrap().id,
        Err(e) => {
            info!("Error retrieving round count: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to retrieve round count");
        }
    };

    let mut states = Vec::new();
    for i in 0..round_count + 1 {
        match get_e3(i).await {
            Ok((state, _key)) => {
                let web_result: WebResultRequest = state.into();
                states.push(web_result);
            }
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
async fn get_round_state_lite(data: web::Json<GetRoundRequest>) -> impl Responder {
    let incoming = data.into_inner();

    match get_e3(incoming.round_id as u64).await {
        Ok((state, _)) => {
            let state_lite: E3StateLite = state.into();
            HttpResponse::Ok().json(state_lite)
        }
        Err(_e) => {
            HttpResponse::InternalServerError().body("Failed to get E3 state")
        }
    }
}
