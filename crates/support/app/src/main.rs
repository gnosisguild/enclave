use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Result as ActixResult};
use anyhow::bail;
use e3_compute_provider::FHEInputs;
use e3_support_host::Risc0Output;
use e3_support_types::{ComputeRequest, ComputeResponse};
use serde::{Deserialize, Deserializer, Serialize};

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
    callback_url: &str,
    e3_id: u64,
    proof: Vec<u8>,
    ciphertext: Vec<u8>,
) -> anyhow::Result<()> {
    let payload = WebhookPayload {
        e3_id,
        ciphertext,
        proof,
    };

    println!("callback_url: {}", callback_url);
    println!("payload: {:?}", payload);

    let _response: serde_json::Value = reqwest::Client::new()
        .post(callback_url)
        .json(&payload)
        .send()
        .await?
        .json()
        .await?;

    println!("✓ Webhook called successfully for E3 {}", e3_id);
    Ok(())
}

async fn run_computation_async(fhe_inputs: FHEInputs) -> anyhow::Result<(Risc0Output, Vec<u8>)> {
    let result =
        tokio::task::spawn_blocking(move || e3_support_host::run_compute(fhe_inputs)).await??;
    Ok(result)
}

async fn handle_webhook_delivery(
    e3_id: u64,
    callback_url: &str,
    proof: Vec<u8>,
    ciphertext: Vec<u8>,
) -> anyhow::Result<()> {
    call_webhook(callback_url, e3_id, proof, ciphertext).await?;
    println!("✓ Webhook sent successfully for E3 {}", e3_id);
    Ok(())
}

async fn process_computation_background(
    e3_id: u64,
    callback_url: &str,
    fhe_inputs: FHEInputs,
) -> anyhow::Result<()> {
    let (risc0_output, ciphertext) = run_computation_async(fhe_inputs).await?;
    let proof: Vec<u8> = risc0_output.seal.into();
    handle_webhook_delivery(e3_id, callback_url, proof, ciphertext).await?;
    println!("✓ Computation completed for E3 {}", e3_id);
    Ok(())
}

async fn handle_compute(req: web::Json<ComputeRequest>) -> ActixResult<HttpResponse> {
    println!("Processing computation...");

    let e3_id = req
        .e3_id
        .ok_or_else(|| actix_web::error::ErrorBadRequest("e3_id is required"))?;

    let callback_url = req
        .callback_url
        .clone()
        .ok_or_else(|| actix_web::error::ErrorBadRequest("callback_url is required"))?;

    let fhe_inputs = FHEInputs {
        params: req.params.clone(),
        ciphertexts: req.ciphertext_inputs.clone(),
    };

    println!("fhe_inputs.params = {:?}", fhe_inputs.params);

    let callback_url = callback_url.clone();

    // Process computation in background
    tokio::spawn(async move {
        if let Err(e) = process_computation_background(e3_id, &callback_url, fhe_inputs).await {
            eprintln!("✗ Background computation failed for E3 {}: {:?}", e3_id, e);
        }
    });

    Ok(HttpResponse::Ok().json(ProcessingResponse {
        status: "processing".to_string(),
        e3_id,
    }))
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let bind_addr = "0.0.0.0:13151";
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .route("/run_compute", web::post().to(handle_compute))
    })
    .bind(bind_addr)?;

    println!("🚀 FHE Compute Service listening on http://{}", bind_addr);

    server.run().await.map_err(Into::into)
}
