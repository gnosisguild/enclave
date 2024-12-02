use std::sync::Arc;
use std::{collections::HashSet, error::Error};
use crate::NetworkPeer;
/// Actor for connecting to an libp2p client via it's mpsc channel interface
/// This Actor should be responsible for
use actix::prelude::*;
use anyhow::anyhow;
use anyhow::Result;
use cipher::Cipher;
use data::Repository;
use enclave_core::{EnclaveEvent, EventBus, EventId, Subscribe};
use libp2p::identity::ed25519;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, info, instrument, trace};
use zeroize::Zeroize;

/// NetworkManager Actor converts between EventBus events and Libp2p events forwarding them to a
/// NetworkPeer for propagation over the p2p network
pub struct NetworkManager {
    bus: Addr<EventBus>,
    tx: Sender<Vec<u8>>,
    sent_events: HashSet<EventId>,
}

impl Actor for NetworkManager {
    type Context = Context<Self>;
}

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "anyhow::Result<()>")]
struct LibP2pEvent(pub Vec<u8>);

impl NetworkManager {
    /// Create a new NetworkManager actor
    pub fn new(bus: Addr<EventBus>, tx: Sender<Vec<u8>>) -> Self {
        Self {
            bus,
            tx,
            sent_events: HashSet::new(),
        }
    }

    pub fn setup(
        bus: Addr<EventBus>,
        tx: Sender<Vec<u8>>,
        mut rx: Receiver<Vec<u8>>,
    ) -> Addr<Self> {
        let addr = NetworkManager::new(bus.clone(), tx).start();

        // Listen on all events
        bus.do_send(Subscribe {
            event_type: String::from("*"),
            listener: addr.clone().recipient(),
        });

        tokio::spawn({
            let addr = addr.clone();

            async move {
                while let Some(msg) = rx.recv().await {
                    addr.do_send(LibP2pEvent(msg))
                }
            }
        });

        addr
    }

    /// Spawn a Libp2p peer and hook it up to this actor
    #[instrument(name = "libp2p", skip_all)]
    pub async fn setup_with_peer(
        bus: Addr<EventBus>,
        peers: Vec<String>,
        cipher: &Arc<Cipher>,
        repository: Repository<Vec<u8>>,
    ) -> Result<(Addr<Self>, tokio::task::JoinHandle<Result<()>>, String)> {
        info!("Reading from repository");
        let mut bytes = if let Some(bytes) = repository.read().await? {
            let decrypted = cipher.decrypt_data(&bytes)?;
            info!("Found keypair in repository");
            decrypted
        } else {
            let kp = libp2p::identity::Keypair::generate_ed25519();
            info!("Generated new keypair {}", kp.public().to_peer_id());
            let innerkp = kp.try_into_ed25519()?;
            let bytes = innerkp.to_bytes().to_vec();

            // We need to clone here so that returned bytes are not zeroized
            repository.write(&cipher.encrypt_data(&mut bytes.clone())?);
            info!("Saved new keypair to repository");
            bytes
        };

        let ed25519_keypair = ed25519::Keypair::try_from_bytes(&mut bytes)?;
        let keypair: libp2p::identity::Keypair = ed25519_keypair.try_into()?;
        let mut peer = NetworkPeer::new(&keypair, peers, None, "tmp-enclave-gossip-topic")?;
        let rx = peer.rx().ok_or(anyhow!("Peer rx already taken"))?;
        let p2p_addr = NetworkManager::setup(bus, peer.tx(), rx);
        let handle = tokio::spawn(async move { Ok(peer.start().await?) });
        Ok((p2p_addr, handle, keypair.public().to_peer_id().to_string()))
    }
}

impl Handler<LibP2pEvent> for NetworkManager {
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

impl Handler<EnclaveEvent> for NetworkManager {
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
                    if let Err(e) = tx.send(bytes).await {
                        error!(error=?e, "Error sending bytes to libp2p");
                    };
                }
                Err(error) => {
                    error!(error=?error, "Could not convert event to bytes for serialization!")
                }
            }
        })
    }
}
