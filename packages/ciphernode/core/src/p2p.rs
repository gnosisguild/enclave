use std::collections::HashSet;

/// Actor for connecting to an libp2p client via it's mpsc channel interface
/// This Actor should be responsible for
/// 1. Sending and Recieving Vec<u8> messages with libp2p
/// 2. Converting between Vec<u8> and EnclaveEvents::Xxxxxxxxx()
/// 3. Broadcasting over the local eventbus
/// 4. Listening to the local eventbus for messages to be published to libp2p
use actix::prelude::*;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    eventbus::{EventBus, Subscribe},
    events::{EnclaveEvent, EventId},
};

pub struct P2p {
    bus: Addr<EventBus>,
    tx: Sender<Vec<u8>>,
    sent_events: HashSet<EventId>,
}

impl Actor for P2p {
    type Context = Context<Self>;
}

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "anyhow::Result<()>")]
struct LibP2pEvent(pub Vec<u8>);

impl P2p {
    pub fn new(bus: Addr<EventBus>, tx: Sender<Vec<u8>>) -> Self {
        Self {
            bus,
            tx,
            sent_events: HashSet::new(),
        }
    }

    /// Start a new P2p actor listening for libp2p messages on the given Receiver and forwarding
    /// them to the actor
    pub fn spawn_and_listen(
        bus: Addr<EventBus>,
        tx: Sender<Vec<u8>>,       // Transmit byte events to the network
        mut rx: Receiver<Vec<u8>>, // Receive byte events from the network
    ) -> Addr<Self> {
        // Create a new Actor
        let p2p = P2p::new(bus.clone(), tx).start();

        // Listen on all events
        bus.do_send(Subscribe {
            event_type: String::from("*"),
            listener: p2p.clone().recipient(),
        });

        // Clone this to go in the spawned future
        let p2p_addr = p2p.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                p2p_addr.do_send(LibP2pEvent(msg))
            }
        });

        // Return the address
        p2p
    }
}

impl Handler<LibP2pEvent> for P2p {
    type Result = anyhow::Result<()>;
    fn handle(&mut self, msg: LibP2pEvent, _: &mut Self::Context) -> Self::Result {
        let LibP2pEvent(bytes) = msg;
        match EnclaveEvent::from_bytes(&bytes) {
            Ok(event) => {
                self.bus.do_send(event.clone());
                self.sent_events.insert(event.into());
            }
            Err(err) => println!("Error: {}", err),
        }
        Ok(())
    }
}

impl Handler<EnclaveEvent> for P2p {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, event: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        let sent_events = self.sent_events.clone();
        let tx = self.tx.clone();
        let evt = event.clone();
        Box::pin(async move {
            let id: EventId = evt.clone().into();
            if sent_events.contains(&id) {
                return;
            }

            match evt.to_bytes() {
                Ok(bytes) => {
                    let _ = tx.send(bytes).await;
                }
                Err(error) => println!("Error: {}", error),
            }
        })
    }
}
