// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_events::ComputeRequestError;
use rayon::ThreadPool;
use std::fmt::Debug;
use std::ops::Deref;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Semaphore, time::sleep};
use tracing::{debug, error, info, warn, Level};

/// A bounded executor for CPU-bound tasks backed by a Rayon thread pool.
#[derive(Debug, Clone)]
pub struct TaskPool {
    semaphore: Arc<Semaphore>,
    thread_pool: Arc<ThreadPool>,
}

impl TaskPool {
    /// Creates a new pool with `threads` worker threads and at most `max_tasks` concurrent tasks.
    pub fn new(threads: usize, max_tasks: usize) -> TaskPool {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("Failed to build thread pool");

        Self {
            thread_pool: Arc::new(thread_pool),
            semaphore: Arc::new(Semaphore::new(max_tasks)),
        }
    }

    pub async fn spawn<OP, T: Debug + Send + 'static>(
        &self,
        task_name: String,
        timed_logs: impl Into<TaskTimeouts>, // [(10, Level::WARN), (30, Level::ERROR)]
        op: OP,
    ) -> Result<T>
    where
        OP: FnOnce() -> T + Send + 'static,
    {
        let timeouts = timed_logs.into();
        // Limit the requests and get them to block
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| ComputeRequestError::SemaphoreError(task_name.to_owned()))?;

        // Warn of long running jobs
        let warning_handle = tokio::spawn(async move {
            for log in timeouts.iter() {
                let delay = Duration::from_secs(log.0);
                sleep(delay).await;
                let msg = format!("Job '{}' has been running for {:?}", task_name, delay);
                match log.1 {
                    Level::WARN => warn!(msg),
                    Level::ERROR => error!(msg),
                    Level::INFO => info!(msg),
                    Level::DEBUG => debug!(msg),
                    _ => (),
                }
            }
        });

        // This uses channels to track pending and complete tasks when
        // using the thread pool
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.thread_pool.spawn(|| {
            let t = op();
            // try to return the result and it's duration note this is sync as it is a oneshot sender.
            if let Err(res) = tx.send(t) {
                error!(
                "There was an error sending the result from the multithread actor: result = {:?}",
                res
            );
            }
        });

        let output = rx.await?;

        warning_handle.abort();

        Ok(output)
    }
}

#[derive(Debug, Clone)]
pub struct TaskTimeouts(pub Vec<TimedLog>);

impl<const N: usize> From<[(u64, Level); N]> for TaskTimeouts {
    fn from(arr: [(u64, Level); N]) -> Self {
        Self(arr.into_iter().map(|(s, l)| TimedLog(s, l)).collect())
    }
}

impl Deref for TaskTimeouts {
    type Target = Vec<TimedLog>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TaskTimeouts {
    pub fn new(logs: Vec<TimedLog>) -> Self {
        Self(logs)
    }
}

impl Default for TaskTimeouts {
    fn default() -> Self {
        [(10, Level::WARN), (30, Level::ERROR)].into()
    }
}

impl From<(u64, Level)> for TimedLog {
    fn from((s, level): (u64, Level)) -> Self {
        Self(s, level)
    }
}

#[derive(Debug, Clone)]
pub struct TimedLog(pub u64, pub tracing::Level);
