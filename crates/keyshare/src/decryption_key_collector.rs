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
    Collecting { total: u64 },
    Finished,
}

pub struct DecryptionKeyCollector {
    todo: HashSet<PartyId>,
    parent: Addr<ThresholdKeyshare>,
    state: CollectorState,
    shares: HashMap<PartyId, Arc<ThresholdShare>>,
}

impl DecryptionKeyCollector {
    pub fn setup(parent: Addr<ThresholdKeyshare>, total: u64) -> Addr<Self> {
        let addr = Self {
            todo: (0..total).collect(),
            parent,
            state: CollectorState::Collecting { total },
            shares: HashMap::new(),
        }
        .start();
        addr
    }
}

impl Actor for DecryptionKeyCollector {
    type Context = actix::Context<Self>;
}

impl Handler<ThresholdShareCreated> for DecryptionKeyCollector {
    type Result = ();
    fn handle(&mut self, msg: ThresholdShareCreated, _: &mut Self::Context) -> Self::Result {
        let start = Instant::now();
        info!("DecryptionKeyCollector: ThresholdShareCreated received by collector");
        if let CollectorState::Finished = self.state {
            info!("DecryptionKeyCollector is finished so ignoring!");
            return;
        };

        let pid = msg.share.party_id;
        info!("DecryptionKeyCollector party id: {}", pid);
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
