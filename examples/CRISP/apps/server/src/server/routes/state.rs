use crate::server::app_data::AppData;
use crate::server::models::GetRoundRequest;
use actix_web::{web, HttpResponse, Responder};
use log::{error, info};

pub fn setup_routes(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/state")
            .route("/result", web::post().to(get_round_result))
            .route("/all", web::get().to(get_all_round_results))
            .route("/lite", web::post().to(get_round_state_lite)),
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
