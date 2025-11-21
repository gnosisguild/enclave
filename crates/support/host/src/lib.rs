// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy_primitives::utils::{parse_ether, parse_units};
use alloy_signer_local::PrivateKeySigner;
use anyhow::{Context, Error, Result};
use bincode::serialize;
use boundless_market::{
    client::ClientError,
    contracts::{boundless_market::MarketError, FulfillmentData},
    request_builder::OfferParams,
    storage::storage_provider_from_env,
    Client,
};
use e3_compute_provider::{
    ComputeInput, ComputeManager, ComputeProvider, ComputeResult, FHEInputs,
};
use e3_user_program::fhe_processor;
use methods::PROGRAM_ELF;
use std::time::{Duration, Instant};
use url::Url;

pub struct BoundlessProvider;

#[derive(Debug, Clone)]
pub enum BoundlessOutput {
    Success {
        result: ComputeResult,
        bytes: Vec<u8>,
        seal: Vec<u8>,
    },
    Error {
        error: String,
    },
}

#[derive(Debug)]
pub enum ComputeError {
    BoundlessFailed(String),
    Other(String),
}

impl ComputeProvider for BoundlessProvider {
    type Output = BoundlessOutput;

    fn prove(&self, input: &ComputeInput) -> Self::Output {
        let is_dev_mode =
            std::env::var("RISC0_DEV_MODE").unwrap_or_else(|_| "0".to_string()) == "1";

        if is_dev_mode {
            println!("Dev mode: Using fake proof");
            fake_prove(input)
        } else {
            println!("Using Boundless for proving");
            tokio::runtime::Handle::current().block_on(boundless_prove(input))
        }
    }
}

fn encode_input(input: &[u8]) -> Result<Vec<u8>, Error> {
    Ok(bytemuck::pod_collect_to_vec(&risc0_zkvm::serde::to_vec(
        input,
    )?))
}

/// Dev mode: return fake proof without executing
fn fake_prove(input: &ComputeInput) -> BoundlessOutput {
    println!("Generating fake proof for dev mode");

    // Execute the program with the input
    let result = input.process(fhe_processor);

    // Serialize the result as journal bytes
    let journal_bytes = bincode::serialize(&result).unwrap_or_default();

    BoundlessOutput::Success {
        result,
        bytes: journal_bytes,
        seal: vec![], // No seal in dev mode
    }
}

/// Boundless proving
async fn boundless_prove(input: &ComputeInput) -> BoundlessOutput {
    println!("Submitting proof request to Boundless...");

    let rpc_url = match std::env::var("RPC_URL")
        .context("RPC_URL not set")
        .and_then(|url| url.parse().context("Invalid RPC_URL"))
    {
        Ok(url) => url,
        Err(e) => {
            return BoundlessOutput::Error {
                error: e.to_string(),
            }
        }
    };

    let private_key: PrivateKeySigner = match std::env::var("PRIVATE_KEY")
        .context("PRIVATE_KEY not set")
        .and_then(|key| key.parse().context("Invalid PRIVATE_KEY"))
    {
        Ok(key) => key,
        Err(e) => {
            return BoundlessOutput::Error {
                error: e.to_string(),
            }
        }
    };

    let storage_provider = match storage_provider_from_env() {
        Ok(provider) => Some(provider),
        Err(e) => {
            eprintln!("Warning: Failed to get storage provider: {}", e);
            None
        }
    };

    let client = match Client::builder()
        .with_rpc_url(rpc_url)
        .with_private_key(private_key)
        .with_storage_provider(storage_provider)
        .build()
        .await
    {
        Ok(client) => client,
        Err(e) => {
            return BoundlessOutput::Error {
                error: format!("Failed to build Boundless client: {}", e),
            }
        }
    };

    let input_bytes = match encode_input(&serialize(input).unwrap()) {
        Ok(bytes) => bytes,
        Err(e) => {
            return BoundlessOutput::Error {
                error: format!("Failed to encode input: {}", e),
            }
        }
    };
    let program_url = std::env::var("PROGRAM_URL").ok();

    let request = if let Some(url) = program_url {
        println!("Using pre-uploaded program: {}", url);
        let parsed_url = match url.parse::<Url>() {
            Ok(url) => url,
            Err(e) => {
                return BoundlessOutput::Error {
                    error: format!("Failed to parse program URL: {}", e),
                }
            }
        };
        match client
            .new_request()
            .with_program_url(parsed_url)
            .context("Failed to create new request")
        {
            Ok(req) => req.with_stdin(input_bytes).with_offer(
                // This auction begins with a flat period, allowing early bidding before the ramp-up begins.
                // The price then increases linearly to 0.03 ETH over 2 mins.
                // The maximum price of 0.03 ETH remains for 8 mins,
                // after which the price drops to 0 ETH for the expiry period of 10 mins.
                OfferParams::builder()
                    .min_price(parse_ether("0.001").unwrap()) // Minimum price in ETH
                    .max_price(parse_ether("0.03").unwrap()) // Maximum price in ETH
                    .timeout(20 * 60) // Total timeout in seconds (20 minutes)
                    .lock_timeout(10 * 60) // Lock timeout in seconds (10 minutes)
                    .ramp_up_period(2 * 60) // Ramp up period in seconds (2 minutes)
                    .lock_collateral(parse_units("5", 18).unwrap()), // 5 ZKC
            ),
            Err(e) => {
                return BoundlessOutput::Error {
                    error: e.to_string(),
                }
            }
        }
    } else {
        println!(
            "Warning: Uploading {}MB program at runtime",
            PROGRAM_ELF.len() / 1_000_000
        );
        client
            .new_request()
            .with_program(PROGRAM_ELF)
            .with_stdin(input_bytes)
    };

    let onchain =
        std::env::var("BOUNDLESS_ONCHAIN").unwrap_or_else(|_| "true".to_string()) == "true";

    let (request_id, expires_at) = match if onchain {
        println!("Submitting onchain...");
        client.submit_onchain(request).await
    } else {
        println!("Submitting offchain...");
        client.submit_offchain(request).await
    } {
        Ok(result) => result,
        Err(e) => {
            return BoundlessOutput::Error {
                error: format!("Failed to submit request: {}", e),
            }
        }
    };

    println!("Request ID: {:x}, waiting for fulfillment...", request_id);

    let fulfillment = match client
        .wait_for_request_fulfillment(request_id, Duration::from_secs(5), expires_at)
        .await
    {
        Ok(fulfillment) => fulfillment,
        Err(ClientError::MarketError(MarketError::RequestHasExpired(_))) => {
            return BoundlessOutput::Error {
                error: format!(
                    "Boundless request expired: no prover picked up the request. Request ID: {:x}",
                    request_id
                ),
            };
        }
        Err(e) => {
            return BoundlessOutput::Error {
                error: format!("Failed to wait for fulfillment: {}", e),
            };
        }
    };

    println!("Proof received from Boundless!");
    let data = fulfillment.data();
    let (_, journal) = match data {
        Ok(FulfillmentData::ImageIdAndJournal(image_id, journal)) => (image_id, journal),
        _ => {
            return BoundlessOutput::Error {
                error: "Invalid fulfillment data".to_string(),
            }
        }
    };

    let decoded_journal: ComputeResult = match bincode::deserialize(&journal) {
        Ok(result) => result,
        Err(e) => {
            return BoundlessOutput::Error {
                error: format!("Failed to decode journal: {}", e),
            }
        }
    };

    BoundlessOutput::Success {
        result: decoded_journal,
        bytes: journal.to_vec(),
        seal: fulfillment.seal.to_vec(),
    }
}

pub fn run_compute(
    params: FHEInputs,
) -> std::result::Result<(BoundlessOutput, Vec<u8>), ComputeError> {
    let boundless_provider = BoundlessProvider;

    let mut provider = ComputeManager::new(
        boundless_provider,
        params.clone(),
        fhe_processor,
        false,
        None,
    );

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

    // Check if the output indicates failure
    match output.0 {
        BoundlessOutput::Success { .. } => Ok(output),
        BoundlessOutput::Error { error } => Err(ComputeError::BoundlessFailed(error)),
    }
}
