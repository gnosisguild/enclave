use crate::{EnclaveEvent, EventBus, Subscribe};
use actix::{Actor, Addr, Context, Handler};
use alloy::primitives::Address;
use base64::prelude::*;
use std::fs;

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
                    "\n\nPUBKEY:\n{}...{}\n\nSaved to scripts/pubkey.b64\n\n",
                    &pubkey_str[..20],
                    &pubkey_str[pubkey_str.len() - 20..]
                );
                fs::write("scripts/pubkey.b64", &pubkey_str).unwrap();
                println!("[{}]: {}", self.name, msg);
            }
            EnclaveEvent::PlaintextAggregated { data, .. } => {
                let output: Vec<u64> = bincode::deserialize(&data.decrypted_output).unwrap();
                println!("\n\nDECRYPTED:\n{:?}\n\n", output);
                println!("[{}]: {}", self.name, msg);
            }
            EnclaveEvent::CiphernodeAdded { data, .. } => {
                println!("[{}]: CiphernodeAdded({})", self.name, Address::from(data.address));
            }
            _ => println!("[{}]: {}", self.name, msg),
        }
    }
}
