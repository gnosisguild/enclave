mod auction;
mod config;
mod routes;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use config::FheKeys;

pub struct AppState {
    pub keys: Arc<FheKeys>,
    pub auctions: Mutex<HashMap<u64, auction::Auction>>,
    pub next_id: AtomicU64,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let keys = Arc::new(FheKeys::generate());
    let state = web::Data::new(AppState {
        keys,
        auctions: Mutex::new(HashMap::new()),
        next_id: AtomicU64::new(1),
    });

    log::info!("Starting auction server on http://0.0.0.0:4001");

    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .app_data(state.clone())
            .route("/health", web::get().to(routes::health))
            .route("/auction/create", web::post().to(routes::create_auction))
            .route("/auction/{id}", web::get().to(routes::get_auction))
            .route("/auction/{id}/bid", web::post().to(routes::submit_bid))
            .route(
                "/auction/{id}/encrypt",
                web::post().to(routes::encrypt_bid_handler),
            )
            .route("/auction/{id}/close", web::post().to(routes::close_auction))
            .route("/auction/{id}/result", web::get().to(routes::get_result))
    })
    .bind("0.0.0.0:4001")?
    .run()
    .await
}
