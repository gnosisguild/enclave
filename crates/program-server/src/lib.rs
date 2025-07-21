// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod types;

use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Result as ActixResult};
use anyhow::Result;
use e3_compute_provider::FHEInputs;
use serde::Serialize;
use std::{future::Future, pin::Pin, sync::Arc};
use types::{ComputeRequest, WebhookPayload};

#[derive(Serialize, Debug)]
struct ProcessingResponse {
    status: String,
    e3_id: u64,
}

type RunnerResult = Result<(Vec<u8>, Vec<u8>)>;
type Runner = dyn Fn(FHEInputs) -> Pin<Box<dyn Future<Output = RunnerResult> + Send>> + Send + Sync;

#[derive(Clone)]
pub struct E3ProgramServerBuilder {
    runner: Arc<Runner>,
    port: Option<u16>,
    host: Option<String>,
    localhost_rewrite: Option<String>,
}

impl E3ProgramServerBuilder {
    /// Create a new builder with a computation callback
    pub fn new<F, Fut>(callback: F) -> Self
    where
        F: Fn(FHEInputs) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RunnerResult> + Send + 'static,
    {
        Self {
            runner: Arc::new(move |inputs| Box::pin(callback(inputs))),
            port: None,
            host: None,
            localhost_rewrite: None,
        }
    }

    /// Set the port number (default: 13151)
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Set the host address (default: "0.0.0.0")
    pub fn with_host<S: Into<String>>(mut self, host: S) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Server will rewrite localhost callbacks to whatever is provided as an argument eg. "host.local". This is usefull when running in a Docker container which does not have direct access to the host
    pub fn with_localhost_rewrite(mut self, rewrite: &str) -> Self {
        self.localhost_rewrite = Some(rewrite.to_string());
        self
    }

    /// Build the E3ProgramServer
    pub fn build(self) -> E3ProgramServer {
        E3ProgramServer {
            runner: self.runner,
            port: self.port.unwrap_or(13151),
            host: self.host.unwrap_or_else(|| "0.0.0.0".to_string()),
            localhost_rewrite: self.localhost_rewrite,
        }
    }
}

#[derive(Clone)]
pub struct E3ProgramServer {
    runner: Arc<Runner>,
    port: u16,
    host: String,
    localhost_rewrite: Option<String>,
}

impl E3ProgramServer {
    /// Create a new builder for E3ProgramServer with a computation callback
    pub fn builder<F, Fut>(callback: F) -> E3ProgramServerBuilder
    where
        F: Fn(FHEInputs) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = RunnerResult> + Send + 'static,
    {
        E3ProgramServerBuilder::new(callback)
    }

    /// Get the configured port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get the configured host
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Get the bind address as a string
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Run the HTTP server
    pub async fn run(&self) -> Result<()> {
        let bind_addr = self.bind_address();
        let config = AppConfig {
            runner: Arc::clone(&self.runner),
            localhost_rewrite: self.localhost_rewrite.clone(),
        };
        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(config.clone()))
                .wrap(Logger::default())
                .route("/run_compute", web::post().to(handle_compute))
                .route("/health", web::get().to(handle_health_check))
                .route("/health", web::head().to(handle_health_check))
        })
        .bind(&bind_addr)?;

        println!("🚀 E3 Program Server listening on http://{}", bind_addr);
        server.run().await.map_err(Into::into)
    }
}

#[derive(Clone)]
pub struct AppConfig {
    pub runner: Arc<Runner>,
    pub localhost_rewrite: Option<String>,
}

async fn call_webhook(
    callback_url: &str,
    e3_id: u64,
    proof: Vec<u8>,
    ciphertext: Vec<u8>,
) -> Result<()> {
    println!("call_webhook()");
    let payload = WebhookPayload {
        e3_id,
        ciphertext,
        proof,
    };
    println!("callback_url: {}", callback_url);
    println!("payload: {:?}", payload);

    reqwest::Client::new()
        .post(callback_url)
        .json(&payload)
        .send()
        .await?
        .error_for_status()?;

    println!("✓ Webhook called successfully for E3 {}", e3_id);
    Ok(())
}

async fn handle_webhook_delivery(
    e3_id: u64,
    callback_url: &str,
    proof: Vec<u8>,
    ciphertext: Vec<u8>,
) -> Result<()> {
    println!("handle_webhook_delivery()");
    call_webhook(callback_url, e3_id, proof, ciphertext).await?;
    println!("✓ Webhook sent successfully for E3 {}", e3_id);
    Ok(())
}

async fn process_computation_background(
    runner: Arc<Runner>,
    e3_id: u64,
    callback_url: &str,
    fhe_inputs: FHEInputs,
) -> Result<()> {
    let (proof, ciphertext) = runner(fhe_inputs).await?;
    println!("computation finished!");
    println!("handling webhook delivery...");
    handle_webhook_delivery(e3_id, callback_url, proof, ciphertext).await?;
    println!("✓ Computation completed for E3 {}", e3_id);
    Ok(())
}

async fn handle_compute(
    config: web::Data<AppConfig>,
    req: web::Json<ComputeRequest>,
) -> ActixResult<HttpResponse> {
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
    let callback_url = if let Some(new_host) = config.localhost_rewrite.clone() {
        callback_url
            .replace("localhost", &new_host)
            .replace("127.0.0.1", &new_host)
    } else {
        callback_url
    };
    println!("callback_url:{}", callback_url);
    let runner = config.runner.clone();
    tokio::spawn(async move {
        if let Err(e) =
            process_computation_background(runner, e3_id, &callback_url, fhe_inputs).await
        {
            eprintln!("✗ Background computation failed for E3 {}: {:?}", e3_id, e);
        }
    });

    Ok(HttpResponse::Ok().json(ProcessingResponse {
        status: "processing".to_string(),
        e3_id,
    }))
}

async fn handle_health_check() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(ProcessingResponse {
        status: "healthy".to_string(),
        e3_id: 0,
    }))
}
