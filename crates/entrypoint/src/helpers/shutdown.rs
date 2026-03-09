// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_ciphernode_builder::CiphernodeHandle;
use e3_events::{prelude::*, Shutdown};
use std::time::Duration;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{error, info};

pub async fn listen_for_shutdown(node: CiphernodeHandle) {
    let bus = node.bus;
    let mut sigterm =
        signal(SignalKind::terminate()).expect("Failed to create SIGTERM signal stream");
    sigterm.recv().await;
    info!("SIGTERM received, initiating graceful shutdown...");

    if let Err(e) = bus.publish_without_context(Shutdown) {
        error!("Shutdown failed to publish! {e}");
    }

    tokio::time::sleep(Duration::from_secs(2)).await;
    info!("Graceful shutdown complete");
}
