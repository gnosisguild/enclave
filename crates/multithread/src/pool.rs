// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_events::ComputeRequestError;
use rayon::ThreadPool;
use std::fmt::Debug;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Semaphore, time::sleep};
use tracing::{error, warn};

#[derive(Debug, Clone)]
pub struct MultithreadThreadpool {
    semaphore: Arc<Semaphore>,
    thread_pool: Arc<ThreadPool>,
}

impl MultithreadThreadpool {
    pub fn new(threads: usize, max_tasks: usize) -> MultithreadThreadpool {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("Failed to build thread pool");

        Self {
            thread_pool: Arc::new(thread_pool),
            semaphore: Arc::new(Semaphore::new(max_tasks)),
        }
    }

    pub async fn spawn<OP, T: Debug + Send + 'static>(&self, task_name: String, op: OP) -> Result<T>
    where
        OP: FnOnce() -> T + Send + 'static,
    {
        // Limit the requests and get them to block
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| ComputeRequestError::SemaphoreError(task_name.to_owned()))?;

        // Warn of long running jobs
        let warning_handle = tokio::spawn(async move {
            sleep(Duration::from_secs(10)).await;
            warn!(
                "Job '{}' has been running for more than 10 seconds",
                task_name
            );
            sleep(Duration::from_secs(30)).await;
            error!(
                "Job '{}' has been running for more than 30 seconds",
                task_name
            );
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
