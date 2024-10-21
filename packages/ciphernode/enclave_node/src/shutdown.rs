use std::time::Duration;

use actix::System;
use tokio::{signal::unix::{signal, SignalKind}, task::JoinHandle};

pub async fn listen_for_shutdown(handle: JoinHandle<()>) {
    let mut sigterm = signal(SignalKind::terminate())
        .expect("Failed to create SIGTERM signal stream");

    sigterm.recv().await;
    println!("SIGTERM received, initiating graceful shutdown...");

    System::current().stop();
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Abort the spawned task
    handle.abort();

    // Wait for the task to finish
    if let Err(e) = handle.await {
        println!("Task error during shutdown: {:?}", e);
    }

    println!("Graceful shutdown complete");
}
