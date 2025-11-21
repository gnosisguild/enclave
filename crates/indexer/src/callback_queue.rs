// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_evm_helpers::threshold_queue::{ThresholdItem, ThresholdQueue};
use eyre::Result;
use std::{future::Future, pin::Pin, sync::Arc};

type AsyncCallback =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

#[derive(Clone)]
/// A callback that has an execute time associated with it
pub struct TimedHandler {
    time: u64,
    handler: AsyncCallback,
}

impl ThresholdItem for TimedHandler {
    type Item = AsyncCallback;

    fn item(&self) -> Self::Item {
        self.handler.clone()
    }

    fn within_threshold(&self, threshold: u64) -> bool {
        self.time <= threshold
    }
}

impl Eq for TimedHandler {}

impl PartialEq for TimedHandler {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl PartialOrd for TimedHandler {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimedHandler {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

#[derive(Clone)]
/// A queue of callbacks that can be executed when a given timestamp has been passed
pub struct CallbackQueue {
    queue: ThresholdQueue<TimedHandler>,
}

impl CallbackQueue {
    /// Create a new queue
    pub fn new() -> Self {
        Self {
            queue: ThresholdQueue::new(),
        }
    }

    /// Push a handler to the queue to be executed at or before the given time.
    pub fn push<F, Fut>(&mut self, time: u64, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.queue.push(TimedHandler {
            time,
            handler: Arc::new(move || Box::pin(handler())),
        })
    }

    /// Execute all pending callbacks up to and including the given time
    pub async fn execute_until_including(&self, time: u64) -> Result<()> {
        let handlers = self.queue.take_until_including(time);
        for handler in handlers {
            handler().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn test_single_callback_executes() {
        let mut queue = CallbackQueue::new();
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        queue.push(100, move || {
            let called = called_clone.clone();
            async move {
                *called.lock().unwrap() = true;
                Ok(())
            }
        });

        queue.execute_until_including(100).await.unwrap();
        assert!(*called.lock().unwrap());
    }

    #[tokio::test]
    async fn test_callback_not_executed_before_threshold() {
        let mut queue = CallbackQueue::new();
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        queue.push(100, move || {
            let called = called_clone.clone();
            async move {
                *called.lock().unwrap() = true;
                Ok(())
            }
        });

        queue.execute_until_including(50).await.unwrap();
        assert!(!*called.lock().unwrap());
    }

    #[tokio::test]
    async fn test_multiple_callbacks_execute() {
        let mut queue = CallbackQueue::new();
        let counter = Arc::new(Mutex::new(0));

        let c1 = counter.clone();
        queue.push(50, move || {
            let c = c1.clone();
            async move {
                *c.lock().unwrap() += 1;
                Ok(())
            }
        });

        let c2 = counter.clone();
        queue.push(100, move || {
            let c = c2.clone();
            async move {
                *c.lock().unwrap() += 1;
                Ok(())
            }
        });

        let c3 = counter.clone();
        queue.push(150, move || {
            let c = c3.clone();
            async move {
                *c.lock().unwrap() += 1;
                Ok(())
            }
        });

        queue.execute_until_including(100).await.unwrap();
        assert_eq!(*counter.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_error_propagation() {
        let mut queue = CallbackQueue::new();

        queue.push(100, || async { Err(eyre::eyre!("test error")) });

        let result = queue.execute_until_including(100).await;
        assert!(result.is_err());
    }
}
