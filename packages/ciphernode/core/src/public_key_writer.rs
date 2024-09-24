use crate::{EnclaveEvent, EventBus, Subscribe};
use actix::{Actor, Addr, Context, Handler};
use alloy::primitives::Address;
use base64::prelude::*;
use std::{env, fs, path::PathBuf};

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
            event_type: "*".to_string(),
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
            let pubkey_str = BASE64_STANDARD.encode(&data.pubkey);

            if let Some(path) = append_relative_path(&self.path) {
                fs::write(path, &pubkey_str).unwrap();
            }
        }
    }
}

fn append_relative_path(relative_path: &str) -> Option<PathBuf> {
    let mut path = env::current_dir().ok()?;
    path.push(relative_path);
    Some(path.canonicalize().ok()?)
}
