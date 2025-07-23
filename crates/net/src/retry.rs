// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use std::{future::Future, time::Duration};
use tokio::time::sleep;
use tracing::{error, warn};

pub enum RetryError {
    Failure(anyhow::Error),
    Retry(anyhow::Error),
}

pub fn to_retry(e: impl Into<anyhow::Error>) -> RetryError {
    RetryError::Retry(e.into())
}

pub const BACKOFF_DELAY: u64 = 500;
pub const BACKOFF_MAX_RETRIES: u32 = 10;

/// Retries an async operation with exponential backoff
///
/// # Arguments
/// * `operation` - Async function to retry
/// * `max_attempts` - Maximum number of retry attempts
/// * `initial_delay_ms` - Initial delay between retries in milliseconds
///
/// # Returns
/// * `Result<()>` - Ok if the operation succeeded, Err if all retries failed
pub async fn retry_with_backoff<F, Fut>(
    operation: F,
    max_attempts: u32,
    initial_delay_ms: u64,
) -> Result<()>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<(), RetryError>>,
{
    let mut current_attempt = 1;
    let mut delay_ms = initial_delay_ms;

    loop {
        match operation().await {
            Ok(_) => return Ok(()),
            Err(re) => {
                match re {
                    RetryError::Retry(e) => {
                        if current_attempt >= max_attempts {
                            return Err(anyhow::anyhow!(
                                "Operation failed after {} attempts. Last error: {}",
                                max_attempts,
                                e
                            ));
                        }

                        warn!(
                            "Attempt {}/{} failed, retrying in {}ms: {}",
                            current_attempt, max_attempts, delay_ms, e
                        );

                        sleep(Duration::from_millis(delay_ms)).await;
                        current_attempt += 1;
                        delay_ms *= 2; // Exponential backoff
                    }
                    RetryError::Failure(e) => {
                        error!("FAILURE!: returning to caller.");
                        return Err(e);
                    }
                }
            }
        }
    }
}
