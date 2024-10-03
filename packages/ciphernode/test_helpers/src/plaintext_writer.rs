use actix::{Actor, Addr, Context, Handler};
use super::write_file_with_dirs;
use enclave_core::{EnclaveEvent, EventBus, Subscribe};

pub struct PlaintextWriter {
    path: String,
}

impl PlaintextWriter {
    pub fn attach(path: &str, bus: Addr<EventBus>) -> Addr<Self> {
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

            println!("Write plaintext to {}", &self.path);
            let contents: Vec<String> = output.iter().map(|&num| num.to_string()).collect();

            // NOTE: panicking is kind of what we want here for now as we don't really need to handle the
            // error yet not knowing if this feature will be in production
            write_file_with_dirs(&self.path, format!("{}", contents.join(",")).as_bytes()).unwrap();
        }
    }
}
