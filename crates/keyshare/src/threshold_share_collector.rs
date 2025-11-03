// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use actix::{Actor, Addr, Handler};
use e3_events::{ThresholdShare, ThresholdShareCreated};
use e3_trbfv::PartyId;
use tracing::info;

use crate::{AllThresholdSharesCollected, ThresholdKeyshare};

pub enum CollectorState {
    Collecting,
    Finished,
}

pub struct ThresholdShareCollector {
    todo: HashSet<PartyId>,
    address: String,
    parent: Addr<ThresholdKeyshare>,
    state: CollectorState,
    shares: HashMap<PartyId, Arc<ThresholdShare>>,
}

impl ThresholdShareCollector {
    pub fn setup(parent: Addr<ThresholdKeyshare>, total: u64, address: &str) -> Addr<Self> {
        let addr = Self {
            todo: (0..total).collect(),
            address: address.to_string(),
            parent,
            state: CollectorState::Collecting,
            shares: HashMap::new(),
        }
        .start();
        addr
    }
}

impl Actor for ThresholdShareCollector {
    type Context = actix::Context<Self>;
}

impl Handler<ThresholdShareCreated> for ThresholdShareCollector {
    type Result = ();
    fn handle(&mut self, msg: ThresholdShareCreated, _: &mut Self::Context) -> Self::Result {
        let start = Instant::now();
        info!("ThresholdShareCollector: ThresholdShareCreated received by collector");
        if let CollectorState::Finished = self.state {
            info!("ThresholdShareCollector is finished so ignoring!");
            return;
        };

        let pid = msg.share.party_id;
        info!(
            "ThresholdShareCollector party id: {} for address: {}",
            pid, self.address
        );
        let Some(_) = self.todo.take(&pid) else {
            info!(
                "Error: {} was not in decryption key collectors ID list",
                pid
            );
            return;
        };
        info!("Inserting... waiting on: {}", self.todo.len());
        self.shares.insert(pid, msg.share);
        if self.todo.len() == 0 {
            info!("We have recieved all the things");
            self.state = CollectorState::Finished;
            let event: AllThresholdSharesCollected = self.shares.clone().into();
            self.parent.do_send(event)
        }
        info!(
            "Finished processing ThresholdShareCreated in {:?}",
            start.elapsed()
        );
    }
}
