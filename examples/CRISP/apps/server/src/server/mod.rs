pub mod blockchain;
pub mod config;
mod database;
mod models;
mod routes;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};

use blockchain::listener::start_listener;

use crate::logger::init_logger;
use config::CONFIG;

#[actix_web::main]
pub async fn start() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_logger();

    tokio::spawn(async {
        if let Err(e) = blockchain::sync::sync_server().await {
            eprintln!("Sync server failed: {:?}", e);
        }
    });

    tokio::spawn(async {
        if let Err(e) = start_listener(
            &CONFIG.ws_rpc_url,
            &CONFIG.enclave_address,
            &CONFIG.ciphernode_registry_address,
        )
        .await
        {
            eprintln!("Listener failed: {:?}", e);
        }
    });
    let bind_addr = "0.0.0.0:4000";
    let server = HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allow_any_header()
            .supports_credentials()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(Logger::new(r#"%a "%r" %s %b %T"#))
            .configure(routes::setup_routes)
    })
    .bind(bind_addr)?;

    println!("'crisp-server' listening on http://{}", bind_addr);

    server.run().await?;

    Ok(())
}
