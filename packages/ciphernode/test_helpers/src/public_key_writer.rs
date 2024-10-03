use super::write_file_with_dirs;
use actix::{Actor, Addr, Context, Handler};
use enclave_core::{EnclaveEvent, EventBus, Subscribe};

pub struct PublicKeyWriter {
    path: String,
}

impl PublicKeyWriter {
    pub fn attach(path: &str, bus: Addr<EventBus>) -> Addr<Self> {
        let addr = Self {
            path: path.to_owned(),
        }
        .start();
        bus.do_send(Subscribe {
            listener: addr.clone().recipient(),
            event_type: "PublicKeyAggregated".to_string(),
        });
        addr
    }
}

impl Actor for PublicKeyWriter {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PublicKeyWriter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        if let EnclaveEvent::PublicKeyAggregated { data, .. } = msg.clone() {
            println!("Write pubkey to {}", &self.path);
            write_file_with_dirs(&self.path, &data.pubkey).unwrap();
        }
    }
}
