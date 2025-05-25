use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Result};
use serde_json::json;

// Hello world handler
async fn hello_world() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "message": "Hello, World!",
        "status": "success"
    })))
}

// Health check endpoint
async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "status": "healthy",
        "service": "actix-web-api"
    })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init();

    println!("Starting Actix Web server on port 4001...");

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .route("/hello", web::get().to(hello_world))
            .route("/health", web::get().to(health_check))
            .route(
                "/",
                web::get().to(|| async {
                    HttpResponse::Ok().json(json!({
                        "message": "Welcome to the Actix Web API",
                        "endpoints": [
                            "GET /hello - Hello world endpoint",
                            "GET /health - Health check endpoint"
                        ]
                    }))
                }),
            )
    })
    .bind("0.0.0.0:4001")?
    .run()
    .await
}
