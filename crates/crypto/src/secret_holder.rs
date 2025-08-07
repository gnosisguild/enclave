// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::watch;
use tokio::time::{sleep, Duration, Instant};
use tracing::info;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// A secure holder for sensitive data that auto-zeroizes on drop
#[derive(ZeroizeOnDrop)]
pub struct SecretHolder {
    data: Zeroizing<Vec<u8>>,
}

impl SecretHolder {
    pub fn new(data: Zeroizing<Vec<u8>>) -> Self {
        Self { data }
    }

    /// Access the secret data immutably
    pub fn access<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Zeroizing<Vec<u8>>) -> R,
    {
        f(&self.data)
    }

    /// Manually purge the secret data
    pub fn purge(&mut self) {
        self.data.zeroize();
    }

    /// Check if the data has been purged (all zeros)
    pub fn is_purged(&self) -> bool {
        self.data.iter().all(|&b| b == 0)
    }

    /// Replace the current secret data with new data
    pub fn update(&mut self, new_data: Zeroizing<Vec<u8>>) {
        // First zeroize the old data
        self.data.zeroize();
        // Then replace with new data
        self.data = new_data;
    }
}

/// A generic timer that can trigger purge operations
#[derive(Clone)]
pub struct PurgeTimer {
    duration: Duration,
    sender: watch::Sender<Instant>,
}

impl PurgeTimer {
    pub fn new(duration: Duration) -> Self {
        let (sender, _) = watch::channel(Instant::now());
        Self { duration, sender }
    }

    /// Start the timer with a purge callback
    pub async fn start<F>(&self, purge_fn: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut receiver = self.sender.subscribe();
        let duration = self.duration;

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = sleep(duration) => {
                        purge_fn();
                        break;
                    }
                    _ = receiver.changed() => {
                        // Timer was reset, continue loop
                    }
                }
            }
        });
    }

    /// Reset the timer
    pub fn reset(&self) {
        let _ = self.sender.send(Instant::now());
    }
}

/// Orchestrates SecretHolder with PurgeTimer for auto-purging on timeout
pub struct TimedSecretHolder {
    secret: Arc<Mutex<SecretHolder>>,
    timer: PurgeTimer,
}

impl TimedSecretHolder {
    pub fn new(data: Zeroizing<Vec<u8>>, timeout: Duration) -> Self {
        let secret = Arc::new(Mutex::new(SecretHolder::new(data)));
        let timer = PurgeTimer::new(timeout);

        // Start the timer with purge callback
        let secret_clone = Arc::clone(&secret);
        tokio::spawn({
            let timer = timer.clone();
            async move {
                timer
                    .start(move || {
                        if let Ok(mut guard) = secret_clone.try_lock() {
                            info!("Purging key data from memory");
                            guard.purge();
                        }
                    })
                    .await;
            }
        });

        Self { secret, timer }
    }

    /// Access the secret data and reset the timer
    pub fn access<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Zeroizing<Vec<u8>>) -> R,
    {
        let guard = self.secret.lock().unwrap();
        if guard.is_purged() {
            return None;
        }

        let result = guard.access(f);
        info!("Resetting key purge timer");
        self.timer.reset();
        Some(result)
    }

    pub fn update(&self, new_data: Zeroizing<Vec<u8>>) {
        let mut guard = self.secret.lock().unwrap();
        guard.update(new_data);
        self.timer.reset();
    }

    /// Check if the secret has been purged
    pub async fn is_purged(&self) -> bool {
        self.secret.lock().unwrap().is_purged()
    }

    /// Manually purge the secret
    pub fn purge(&self) {
        self.secret.lock().unwrap().purge();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_secret_holder() {
        let mut holder = SecretHolder::new(Zeroizing::new(vec![1, 2, 3, 4]));

        // Test access
        let sum = holder.access(|data| data.iter().sum::<u8>());
        assert_eq!(sum, 10);

        // Test purge
        holder.purge();
        assert!(holder.is_purged());
    }

    #[tokio::test]
    async fn test_timed_secret_holder() {
        let holder =
            TimedSecretHolder::new(Zeroizing::new(vec![1, 2, 3, 4]), Duration::from_millis(50));

        // Test access resets timer
        let sum = holder.access(|data| data.iter().sum::<u8>());
        assert_eq!(sum, Some(10));

        // Wathe value it for purge
        sleep(Duration::from_millis(100)).await;
        // assert!(holder.is_purged().await);

        // Access after purge returns None
        let result = holder.access(|data| data.len());
        assert_eq!(result, None);
    }
}
