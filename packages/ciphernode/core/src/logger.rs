use crate::{EnclaveEvent, EventBus, Subscribe};
use actix::{Actor, Addr, Context, Handler};
use base64::prelude::*;

pub struct SimpleLogger {
    name: String,
}

impl SimpleLogger {
    pub fn attach(name: &str, bus: Addr<EventBus>) -> Addr<Self> {
        let addr = Self {
            name: name.to_owned(),
        }
        .start();
        bus.do_send(Subscribe {
            listener: addr.clone().recipient(),
            event_type: "*".to_string(),
        });
        println!("[{}]: READY", name);
        addr
    }
}

impl Actor for SimpleLogger {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for SimpleLogger {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        match msg.clone() {
            EnclaveEvent::PublicKeyAggregated { data, .. } => {
                let pubkey_str = BASE64_STANDARD.encode(&data.pubkey);
                println!(
                    "\n[{}]: PUBKEY: {}...{}\n",
                    self.name,
                    &pubkey_str[..20],
                    &pubkey_str[pubkey_str.len() - 20..]
                );
                println!("[{}]: {}", self.name, msg);
            }
            EnclaveEvent::CiphernodeAdded { data, .. } => {
                println!("[{}]: CiphernodeAdded({})", self.name, data.address);
            },
            EnclaveEvent::E3Requested { data,.. } => {
                println!("[{}]: E3Requested(e3_id: {}, threshold_m: {} , seed: {})", self.name, data.e3_id, data.threshold_m, data.seed)
            }
            _ => println!("[{}]: {}", self.name, msg),
        }
    }
}
