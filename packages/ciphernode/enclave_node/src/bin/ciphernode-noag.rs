use enclave_core::Actor;
use enclave_core::Ciphernode;
use enclave_core::CiphernodeSelector;
use enclave_core::Data;
use enclave_core::EventBus;
use enclave_core::Fhe;
use enclave_core::P2p;
use enclave_core::SimpleLogger;
use std::error::Error;

/// Note this is untestable so it may break as we change our API
#[actix_rt::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let fhe = Fhe::try_default()?.start();
    let bus = EventBus::new(true).start();
    let data = Data::new(true).start(); // TODO: Use a sled backed Data Actor
    CiphernodeSelector::attach(bus.clone());
    SimpleLogger::attach(bus.clone());
    Ciphernode::attach(bus.clone(), fhe.clone(), data.clone());
    let (_, h) = P2p::spawn_libp2p(bus.clone())?;
    println!("Ciphernode");
    let _ = tokio::join!(h);
    Ok(())
}
