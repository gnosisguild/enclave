use std::error::Error;

use enclave_core::Actor;
use enclave_core::Ciphernode;
use enclave_core::Data;
use enclave_core::EventBus;
use enclave_core::Fhe;
use enclave_core::P2p;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let fhe = Fhe::try_default()?.start();
    let bus = EventBus::new(true).start();
    let data = Data::new(true).start(); // TODO: Use a sled backed Data Actor
    let _node = Ciphernode::new(bus.clone(), fhe.clone(), data.clone()).start();
    let (_, h) = P2p::spawn_libp2p(bus.clone())?;
    println!("Ciphernode");
    let _ = tokio::join!(h);
    Ok(())
}
