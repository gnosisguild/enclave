use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Result};
use compute_provider::FHEInputs;
use program_client::{ComputeRequest, ComputeResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;

// Run compute handler
async fn run_compute(req: web::Json<ComputeRequest>) -> Result<HttpResponse> {
    let fhe_inputs = FHEInputs {
        params: req.params,
        ciphertexts: req.ciphertext_inputs,
    };
    let (risc0_output, ciphertext) =
        tokio::task::spawn_blocking(move || voting_host::run_compute(fhe_inputs))
            .await
            .map_err(|e| {
                eprintln!("Task spawn error: {:?}", e);
                actix_web::error::ErrorInternalServerError("Task execution failed")
            })?
            .map_err(|e| {
                eprintln!("Compute function error: {:?}", e);
                actix_web::error::ErrorInternalServerError("Computation failed")
            })?;

    let proof: Vec<u8> = risc0_output.seal.into();
    let response = ComputeResponse { ciphertext, proof };

    Ok(HttpResponse::Ok().json(response))
}

// Health check endpoint
async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "status": "healthy",
        "service": "enclave_program"
    })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .route("/run_compute", web::post().to(run_compute))
    })
    .bind("0.0.0.0:4001")?
    .run()
    .await
}
