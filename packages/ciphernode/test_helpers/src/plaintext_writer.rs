use std::path::PathBuf;

use super::write_file_with_dirs;
use actix::{Actor, Addr, Context, Handler};
use events::{EnclaveEvent, EventBus, Subscribe};
use tracing::info;

pub struct PlaintextWriter {
    path: PathBuf,
}

impl PlaintextWriter {
    pub fn attach(path: &PathBuf, bus: Addr<EventBus<EnclaveEvent>>) -> Addr<Self> {
        let addr = Self {
            path: path.to_owned(),
        }
        .start();
        bus.do_send(Subscribe {
            listener: addr.clone().recipient(),
            event_type: "PlaintextAggregated".to_string(),
        });
        addr
    }
}

impl Actor for PlaintextWriter {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PlaintextWriter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        if let EnclaveEvent::PlaintextAggregated { data, .. } = msg.clone() {
            let output: Vec<u64> = bincode::deserialize(&data.decrypted_output).unwrap();

            info!(path = ?&self.path, "Writing Plaintext To Path");
            let contents: Vec<String> = output.iter().map(|&num| num.to_string()).collect();

            write_file_with_dirs(&self.path, format!("{}", contents.join(",")).as_bytes()).unwrap();
        }
    }
}
