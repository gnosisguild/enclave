use crate::{EnclaveEvent, EventBus, Subscribe};
use actix::{Actor, Addr, Context, Handler};
use base64::prelude::*;
use fhe::bfv::PublicKey;
use fhe_traits::DeserializeParametrized;
use std::{fs, sync::Arc};

pub struct SimpleLogger;

impl SimpleLogger {
    pub fn attach(bus: Addr<EventBus>) -> Addr<Self> {
        let addr = Self.start();
        bus.do_send(Subscribe {
            listener: addr.clone().recipient(),
            event_type: "*".to_string(),
        });
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
                println!("\n\nPUBKEY:\n{}\n\n", pubkey_str);
                fs::write("scripts/pubkey.b64", &pubkey_str).unwrap();
                println!("{}", msg);
            }
            EnclaveEvent::PlaintextAggregated { data, .. } => {
                let output: Vec<u64> = bincode::deserialize(&data.decrypted_output).unwrap();
                println!("\n\nDECRYPTED:\n{:?}\n\n", output);
            }
            _ => println!("{}", msg),
        }
    }
}
