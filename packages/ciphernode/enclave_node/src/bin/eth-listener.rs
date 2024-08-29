use enclave_core::Actor;
use enclave_core::EventBus;

use eth::StartListening;
use eth::{AddListener, ContractManager, AddEventHandler};
use std::error::Error;
use tokio::signal;
use alloy::{primitives::address, sol};

sol! {
    #[derive(Debug)]
    event TestingEvent(uint256 e3Id, bytes input);
}

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bus = EventBus::new(true).start();

    let manager = ContractManager::attach(bus.clone(), "ws://127.0.0.1:8545").await;
    let listener = manager
        .send(AddListener {
            contract_address: address!("e7f1725E7734CE288F8367e1Bb143E90bb3F0512"),
        })
        .await
        .unwrap();

    listener.send(AddEventHandler::<TestingEvent>::new()).await.unwrap();
    listener.do_send(StartListening); // or manager.do_send(StartListening) if multiple listeners

    signal::ctrl_c().await.unwrap();
    
    Ok(())
}
