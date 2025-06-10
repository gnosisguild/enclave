use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Result as ActixResult};
use anyhow::bail;
use e3_compute_provider::FHEInputs;
use e3_support_types::{ComputeRequestPayload, ComputeResponse};
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

async fn handle_compute(req: web::Json<ComputeRequestPayload>) -> ActixResult<HttpResponse> {
    // TODO: process this in a spawn so that we return early and allow webhook instead of
    // processing sequentially
    println!("Processing computation...");
    let fhe_inputs = FHEInputs {
        params: req.params.clone(),
        ciphertexts: req.ciphertext_inputs.clone(),
    };

    let (risc0_output, ciphertext) =
        tokio::task::spawn_blocking(move || e3_support_host::run_compute(fhe_inputs))
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

    match (req.e3_id, &req.callback_url) {
        (Some(e3_id), Some(callback_url)) => {
            let callback_url = callback_url.clone();
            tokio::spawn(async move {
                if let Err(e) = call_webhook(&callback_url, e3_id, proof, ciphertext).await {
                    eprintln!("✗ Webhook failed for E3 {}: {}", e3_id, e);
                }
            });

            Ok(HttpResponse::Ok().json(ProcessingResponse {
                status: "processing".to_string(),
                e3_id,
            }))
        }
        (Some(e3_id), None) => {
            println!("⚠️ E3 {} completed but no callback URL provided", e3_id);
            let response = ComputeResponse { ciphertext, proof };
            Ok(HttpResponse::Ok().json(response))
        }
        (None, _) => {
            let response = ComputeResponse { ciphertext, proof };
            Ok(HttpResponse::Ok().json(response))
        }
    }
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
