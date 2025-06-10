use actix::Recipient;
use anyhow::Result;
use e3_events::{EnclaveEvent, Shutdown};
use std::time::Duration;
use tokio::{
    select,
    signal::unix::{signal, SignalKind},
    task::JoinHandle,
};
use tracing::{error, info};

pub async fn listen_for_shutdown(bus: Recipient<EnclaveEvent>, mut handle: JoinHandle<Result<()>>) {
    let mut sigterm =
        signal(SignalKind::terminate()).expect("Failed to create SIGTERM signal stream");
    select! {
        _ = sigterm.recv() => {
            info!("SIGTERM received, initiating graceful shutdown...");

            // Stop the actor system
            let _ = bus.send(EnclaveEvent::from(Shutdown)).await;

            // Abort the spawned task
            handle.abort();

            // Wait for all actor processes to disconnect
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Wait for the task to finish
            let _ = handle.await;

            info!("Graceful shutdown complete");

        }
        result = &mut handle => {
            match result {
                Ok(Ok(_)) => {
                    info!("Completed");
                }
                Ok(Err(e)) => {
                    error!("Failed: {}", e);
                }
                Err(e) => {
                    error!("Panicked: {}", e);
                }
            }
        }
    }
}
