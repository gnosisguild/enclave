use std::error::Error;

use enclave_core::Actor;
use enclave_core::CommitteeRequested;
use enclave_core::E3id;
use enclave_core::EnclaveEvent;
use enclave_core::EventBus;
use enclave_core::P2p;
use tokio::{
    self,
    io::{self, AsyncBufReadExt, BufReader},
};

/// Note this is untestable so it may break as we change our API
#[actix_rt::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bus = EventBus::new(true).start();
    let (_, t1) = P2p::spawn_libp2p(bus.clone())?;
    let mut stdin = BufReader::new(io::stdin()).lines();
    let t2 = tokio::spawn(async move {
        let mut id: u32 = 1000;
        while let Ok(Some(line)) = stdin.next_line().await {
            match line.as_str() {
                "test" => {
                    id += 1;
                    bus.do_send(EnclaveEvent::from(CommitteeRequested {
                        e3_id: E3id::from(id),
                        nodecount: 3,
                        threshold: 3,
                        sortition_seed: 100,
                    }));
                }
                _ => println!("Unknown command"),
            }
        }
    });

    let _ = tokio::join!(t1, t2);
    Ok(())
}
