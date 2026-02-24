// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;

use anyhow::Result;
use e3_events::{EnclaveEvent, Unsequenced};
use tokio::sync::{broadcast, mpsc};

use crate::events::{NetCommand, NetEvent};

pub enum BatchCursor {
    Done,
    Next(u128),
}

pub struct EventBatch {
    pub events: Vec<EnclaveEvent<Unsequenced>>,
    pub next: BatchCursor,
}

pub async fn fetch_net_events_since(
    _net_cmds: mpsc::Sender<NetCommand>,
    _net_events: Arc<broadcast::Receiver<NetEvent>>,
    _since_hlc: u128,
    _limit: u16,
) -> Result<EventBatch> {
    todo!("fetch_net_events_since implementation")
}
