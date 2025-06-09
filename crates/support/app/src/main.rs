use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Result as ActixResult};
use e3_compute_provider::FHEInputs;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct WebhookConfig {
    pub json_rpc_server: String,
    pub chain: String,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub webhook_config: Option<WebhookConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ComputeRequest {
    pub params: Vec<u8>,
    pub ciphertext_inputs: Vec<(Vec<u8>, u64)>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ComputeResponse {
    pub ciphertext: Vec<u8>,
    pub proof: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct ComputeRequestPayload {
    pub e3_id: Option<u64>,
    pub params: Vec<u8>,
    pub ciphertext_inputs: Vec<(Vec<u8>, u64)>,
}

#[derive(Serialize, Debug)]
struct WebhookPayload {
    pub e3_id: u64,
    pub ciphertext: Vec<u8>,
    pub proof: Vec<u8>,
}

#[derive(Serialize, Debug)]
struct ProcessingResponse {
    status: String,
    e3_id: u64,
}

async fn call_webhook(
    config: &WebhookConfig,
    e3_id: u64,
    proof: Vec<u8>,
    ciphertext: Vec<u8>,
) -> anyhow::Result<()> {
    let payload = WebhookPayload {
        e3_id,
        ciphertext,
        proof,
    };

    let _response: serde_json::Value = reqwest::Client::new()
        .post(&config.json_rpc_server)
        .json(&payload)
        .send()
        .await?
        .json()
        .await?;

    println!("✓ Webhook called successfully for E3 {}", e3_id);
    Ok(())
}

// Main compute handler
async fn handle_compute(
    req: web::Json<ComputeRequestPayload>,
    data: web::Data<Arc<RwLock<AppState>>>,
) -> ActixResult<HttpResponse> {
    let fhe_inputs = FHEInputs {
        params: req.params.clone(),
        ciphertexts: req.ciphertext_inputs.clone(),
    };

    let (risc0_output, ciphertext) = tokio::task::spawn_blocking(move || {
        e3_support_host::run_compute(fhe_inputs)
    })
    .await
    .map_err(|e| {
        eprintln!("Task spawn failed: {:?}", e);
        actix_web::error::ErrorInternalServerError("Task execution failed")
    })?
    .map_err(|e| {
        eprintln!("Computation failed: {:?}", e);
        actix_web::error::ErrorInternalServerError("Computation failed")
    })?;

    let proof: Vec<u8> = risc0_output.seal.into();

    match req.e3_id {
        Some(e3_id) => {
            // Async mode
            let state = data.read().await;
            if let Some(webhook_config) = &state.webhook_config {
                let config = webhook_config.clone();
                tokio::spawn(async move {
                    if let Err(e) = call_webhook(&config, e3_id, proof, ciphertext).await {
                        eprintln!("✗ Webhook failed for E3 {}: {}", e3_id, e);
                    }
                });
            }

            Ok(HttpResponse::Ok().json(ProcessingResponse {
                status: "processing".to_string(),
                e3_id,
            }))
        }
        None => {
            // Sync mode
            let response = ComputeResponse { ciphertext, proof };
            Ok(HttpResponse::Ok().json(response))
        }
    }
}

pub async fn start_with_webhook(json_rpc_server: &str, chain: &str) -> anyhow::Result<()> {
    let webhook_config = WebhookConfig {
        json_rpc_server: json_rpc_server.to_string(),
        chain: chain.to_string(),
    };

    let state = Arc::new(RwLock::new(AppState {
        webhook_config: Some(webhook_config),
    }));

    start_server_with_state(state).await
}

async fn start_server_with_state(state: Arc<RwLock<AppState>>) -> anyhow::Result<()> {
    env_logger::init();
    
    let bind_addr = "0.0.0.0:4001";
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(Logger::default())
            .route("/run_compute", web::post().to(handle_compute))
    })
    .bind(bind_addr)?;

    println!("🚀 FHE Compute Service listening on http://{}", bind_addr);
    
    server.run().await.map_err(Into::into)
}

pub async fn start_standalone() -> anyhow::Result<()> {
    let state = Arc::new(RwLock::new(AppState {
        webhook_config: None,
    }));

    start_server_with_state(state).await
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    start_standalone().await
}