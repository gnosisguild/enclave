use std::path::PathBuf;

use actix::{Actor, Handler, Message};
use commitlog::{CommitLog, LogOptions};
use e3_events::{trap, BusHandle, EType};

#[derive(Message)]
#[rtype(result = "()")]
pub struct Persist {
    value: Vec<u8>,
}

pub struct EventLog {
    log: CommitLog,
    bus: BusHandle,
}

impl EventLog {
    pub fn new(path: &PathBuf, bus: &BusHandle) -> Self {
        let opts = LogOptions::new(path);
        let log = CommitLog::new(opts).unwrap();

        Self {
            log,
            bus: bus.clone(),
        }
    }
}

impl Actor for EventLog {
    type Context = actix::Context<Self>;
}

impl Handler<Persist> for EventLog {
    type Result = ();
    fn handle(&mut self, msg: Persist, ctx: &mut Self::Context) -> Self::Result {
        trap(EType::Data, &self.bus, || {
            self.log.append_msg(&msg.value)?;
            Ok(())
        })
    }
}
