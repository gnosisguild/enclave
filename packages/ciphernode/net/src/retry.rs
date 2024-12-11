use anyhow::Result;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

pub struct RetryConfig {
    max_retries: u32,
    initial_delay: Duration,
    max_delay: Duration,
    backoff_factor: f32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_factor: 2.0,
        }
    }
}

impl RetryConfig {
    pub fn new(
        max_retries: u32,
        initial_delay: Duration,
        max_delay: Duration,
        backoff_factor: f32,
    ) -> Self {
        Self {
            max_retries,
            initial_delay,
            max_delay,
            backoff_factor,
        }
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    pub fn with_backoff_factor(mut self, factor: f32) -> Self {
        self.backoff_factor = factor;
        self
    }
}

pub struct RetryHandler {
    config: RetryConfig,
}

impl RetryHandler {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    pub async fn retry<F, Fut, T>(&self, operation: F, context: &str) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut attempts = 0;
        let mut delay = self.config.initial_delay;

        loop {
            info!("{}...", context);
            match operation().await {
                Ok(value) => {
                    if attempts > 0 {
                        debug!(
                            "Operation '{}' succeeded after {} attempts",
                            context,
                            attempts + 1
                        );
                    }
                    return Ok(value);
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= self.config.max_retries {
                        return Err(anyhow::anyhow!(
                            "Operation '{}' failed after {} attempts. Last error: {}",
                            context,
                            attempts,
                            e
                        ));
                    }

                    warn!(
                        "Attempt {} for '{}' failed: {}. Retrying in {:?}",
                        attempts, context, e, delay
                    );

                    sleep(delay).await;

                    // Calculate next delay with exponential backoff
                    delay = Duration::from_secs_f32(
                        (delay.as_secs_f32() * self.config.backoff_factor)
                            .min(self.config.max_delay.as_secs_f32()),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::bail;

    use super::*;

    #[tokio::test]
    async fn test_retry_handler() -> Result<()> {
        let retry_handler = RetryHandler::new(RetryConfig::default());

        let counter = std::sync::atomic::AtomicU32::new(0);

        // Example of retrying a fallible operation
        let result = retry_handler
            .retry(
                || async {
                    let current = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if current < 2 {
                        bail!("Not ready yet")
                    }
                    Ok("Success!")
                },
                "test operation",
            )
            .await?;

        assert_eq!(result, "Success!");
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);

        Ok(())
    }
}
