use actix::Recipient;
use events::{EnclaveEvent, Shutdown};
use std::time::Duration;
use tokio::signal::unix::{signal, SignalKind};
use tracing::info;

pub async fn listen_for_shutdown(bus: Recipient<EnclaveEvent>) {
    let mut sigterm =
        signal(SignalKind::terminate()).expect("Failed to create SIGTERM signal stream");

    sigterm.recv().await;
    info!("SIGTERM received, initiating graceful shutdown...");

    // Stop the actor system
    let _ = bus.send(EnclaveEvent::from(Shutdown)).await;


    // Wait for all actor processes to disconnect
    tokio::time::sleep(Duration::from_secs(2)).await;

    info!("Graceful shutdown complete");
}
