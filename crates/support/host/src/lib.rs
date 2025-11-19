// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{Context, Result};
use bincode::serialize;
use e3_compute_provider::{
    ComputeInput, ComputeManager, ComputeProvider, ComputeResult, FHEInputs,
};
use alloy_signer_local::PrivateKeySigner;
use e3_user_program::fhe_processor;
use methods::PROGRAM_ELF;
use boundless_market::{Client, storage::storage_provider_from_env, contracts::FulfillmentData};
use url::Url;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub struct BoundlessProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundlessOutput {
    pub result: ComputeResult,
    pub bytes: Vec<u8>,
    pub seal: Vec<u8>,
}

impl ComputeProvider for BoundlessProvider {
    type Output = BoundlessOutput;

    fn prove(&self, input: &ComputeInput) -> Self::Output {
        let is_dev_mode = std::env::var("RISC0_DEV_MODE")
            .unwrap_or_else(|_| "0".to_string()) == "1";
        
        if is_dev_mode {
            println!("Dev mode: Using fake proof");
            fake_prove(input)
        } else {
            println!("Using Boundless for proving");
            tokio::runtime::Handle::current()
                .block_on(boundless_prove(input))
                .expect("Boundless proving failed")
        }
    }
}

/// Dev mode: return fake proof without executing
fn fake_prove(input: &ComputeInput) -> BoundlessOutput {
    println!("Generating fake proof for dev mode");
    
    // Execute the program with the input
    let result = input.process(fhe_processor);
    
    // Serialize the result as journal bytes
    let journal_bytes = bincode::serialize(&result).unwrap_or_default();
    
    BoundlessOutput {
        result,
        bytes: journal_bytes,
        seal: vec![], // No seal in dev mode
    }
}

/// Boundless proving
async fn boundless_prove(input: &ComputeInput) -> Result<BoundlessOutput> {
    println!("Submitting proof request to Boundless...");

    let rpc_url = std::env::var("RPC_URL")
        .context("RPC_URL not set")?
        .parse()
        .context("Invalid RPC_URL")?;
    
    let private_key: PrivateKeySigner = std::env::var("PRIVATE_KEY")
        .context("PRIVATE_KEY not set")?
        .parse()
        .context("Invalid PRIVATE_KEY")?;

    let client = Client::builder()
        .with_rpc_url(rpc_url)
        .with_private_key(private_key)
        .with_storage_provider(Some(storage_provider_from_env()?))
        .build()
        .await
        .context("Failed to build Boundless client")?;

    let input_bytes = serialize(input)?;
    let program_url = std::env::var("PROGRAM_URL").ok();
    
    let request = if let Some(url) = program_url {
        println!("Using pre-uploaded program: {}", url);
        client.new_request()
            .with_program_url(url.parse::<Url>().context("Failed to parse program URL")?)
            .context("Failed to create new request")?
            .with_stdin(input_bytes)
    } else {
        println!("Warning: Uploading {}MB program at runtime", PROGRAM_ELF.len() / 1_000_000);
        client.new_request()
            .with_program(PROGRAM_ELF)
            .with_stdin(input_bytes)
    };

    let onchain = std::env::var("BOUNDLESS_ONCHAIN")
        .unwrap_or_else(|_| "true".to_string()) == "true";
    
    let (request_id, expires_at) = if onchain {
        println!("Submitting onchain...");
        client.submit_onchain(request).await?
    } else {
        println!("Submitting offchain...");
        client.submit_offchain(request).await?
    };

    println!("Request ID: {:x}, waiting for fulfillment...", request_id);

    let fulfillment = client
        .wait_for_request_fulfillment(
            request_id,
            Duration::from_secs(5),
            expires_at,
        )
        .await
        .context("Failed to wait for fulfillment")?;

    println!("Proof received from Boundless!");
    let data = fulfillment.data();
    let (_, journal) = match data {
        Ok(FulfillmentData::ImageIdAndJournal(image_id, journal)) => (image_id, journal),
        _ => return Err(anyhow::anyhow!("Invalid fulfillment data")),
    };

    let decoded_journal: ComputeResult = bincode::deserialize(&journal)
        .context("Failed to decode journal")?;

    Ok(BoundlessOutput {
        result: decoded_journal,
        bytes: journal.to_vec(),
        seal: fulfillment.seal.to_vec(),
    })
}

pub fn run_compute(params: FHEInputs) -> Result<(BoundlessOutput, Vec<u8>)> {
    let boundless_provider = BoundlessProvider;

    let mut provider = ComputeManager::new(boundless_provider, params, fhe_processor, false, None);

    // Start timer
    let start_time = Instant::now();

    let output = provider.start();

    // Capture end time and calculate the duration
    let elapsed_time = start_time.elapsed();

    // Convert the elapsed time to minutes and seconds
    let minutes = elapsed_time.as_secs() / 60;
    let seconds = elapsed_time.as_secs() % 60;

    println!(
        "Prove function execution time: {} minutes and {} seconds",
        minutes, seconds
    );

    Ok(output)
}
