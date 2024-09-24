use crate::{EnclaveEvent, EventBus, Subscribe};
use actix::{Actor, Addr, Context, Handler};
use base64::prelude::*;
use std::fs;
use std::io::Write;

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

        println!("PublicKeyWriter attached to path {}", path);
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

            println!("Write pubkey to {}", &self.path);
            write_file_with_dirs(&self.path, &pubkey_str).unwrap();
        }
    }
}

fn write_file_with_dirs(relative_path: &str, content: &str) -> std::io::Result<()> {
    // Get the current working directory
    let cwd = std::env::current_dir()?;

    // Create an absolute path by joining the cwd and the relative path
    let abs_path = cwd.join(relative_path);

    // Ensure the directory structure exists
    if let Some(parent) = abs_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Open the file (creates it if it doesn't exist) and write the content
    let mut file = fs::File::create(&abs_path)?;
    file.write_all(content.as_bytes())?;

    println!("File written successfully: {:?}", abs_path);
    Ok(())
}
