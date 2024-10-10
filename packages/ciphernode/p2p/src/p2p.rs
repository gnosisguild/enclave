use std::{collections::HashSet, error::Error};

use crate::libp2p_router::EnclaveRouter;
/// Actor for connecting to an libp2p client via it's mpsc channel interface
/// This Actor should be responsible for
use actix::prelude::*;
use tokio::sync::mpsc::{Receiver, Sender};

use enclave_core::{EnclaveEvent, EventBus, EventId, Subscribe};
use tracing::{error, trace};

/// P2p Actor converts between EVentBus events and Libp2p events
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
    /// Create a new P2p actor
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

    /// Spawn a Libp2p instance. Calls spawn and listen
    pub fn spawn_libp2p(
        bus: Addr<EventBus>,
    ) -> Result<(Addr<Self>, tokio::task::JoinHandle<()>), Box<dyn Error>> {
        let (mut libp2p, tx, rx) = EnclaveRouter::new()?;
        libp2p.connect_swarm("mdns".to_string())?;
        libp2p.join_topic("enclave-keygen-01")?;

        let p2p_addr = Self::spawn_and_listen(bus, tx, rx);
        let handle = tokio::spawn(async move { libp2p.start().await.unwrap() });
        Ok((p2p_addr, handle))
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
            Err(err) => error!(error=?err, "Could not create EnclaveEvent from Libp2p Bytes!"),
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

            // if we have seen this event before dont rebroadcast
            if sent_events.contains(&id) {
                trace!(evt_id=%id,"Have seen event before not rebroadcasting!");
                return;
            }

            // Ignore events that should be considered local
            if evt.is_local_only() {
                trace!(evt_id=%id,"Local events should not be rebroadcast so ignoring");
                return;
            }

            match evt.to_bytes() {
                Ok(bytes) => {
                    let _ = tx.send(bytes).await;
                }
                Err(error) => {
                    error!(error=?error, "Could not convert event to bytes for serialization!")
                }
            }
        })
    }
}
