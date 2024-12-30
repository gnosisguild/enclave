use actix::Recipient;
use anyhow::Result;
use enclave_core::{EnclaveEvent, Shutdown};
use std::time::Duration;
use tokio::{
    signal::unix::{signal, SignalKind},
    task::JoinHandle,
};
use tracing::info;

pub async fn listen_for_shutdown(bus: Recipient<EnclaveEvent>, handle: JoinHandle<Result<()>>) {
    let mut sigterm =
        signal(SignalKind::terminate()).expect("Failed to create SIGTERM signal stream");

    sigterm.recv().await;
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
