// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod cid;
mod correlator;
mod dialer;
pub mod direct_requester;
pub mod direct_responder;
mod document_publisher;
pub mod events;
mod net_event_batch;
mod net_event_buffer;
mod net_event_translator;
mod net_interface;
mod net_interface_handle;
mod net_sync_manager;
mod repo;

use std::sync::Arc;

use actix::Recipient;
use anyhow::bail;
use anyhow::Result;
pub use cid::ContentHash;
pub use document_publisher::*;
use e3_crypto::Cipher;
use e3_data::Repository;
use e3_events::{run_once, BusHandle, EffectsEnabled, EventStoreQueryBy, EventSubscriber, TsAgg};
use net_event_buffer::NetEventBuffer;
pub use net_event_translator::*;
pub use net_interface::*;
pub use net_interface_handle::*;
use net_sync_manager::NetSyncManager;
pub use repo::*;
use tracing::error;
use tracing::{info, instrument};

pub async fn setup_libp2p_keypair(
    repository: Repository<Vec<u8>>,
    cipher: &Arc<Cipher>,
) -> Result<Libp2pKeypair> {
    // Get existing keypair or generate a new one
    let mut bytes = match repository.read().await? {
            Some(bytes) => {
                info!("Found keypair in repository");
                cipher.decrypt_data(&bytes)?
            }
            None => bail!("No network keypair found in repository, please generate a new one using `enclave net generate-key`"),
        };
    Libp2pKeypair::try_from_bytes(&mut bytes)
}

pub fn setup_net_interface(
    topic: &str,
    keypair: Libp2pKeypair,
    peers: Vec<String>,
    quic_port: u16,
) -> Result<NetInterfaceHandle> {
    let mut interface = Libp2pNetInterface::new(keypair, peers, Some(quic_port), topic)?;

    let handle = interface.handle();

    actix::spawn(async move {
        if let Err(e) = interface.start().await {
            error!("{e}");
        }
    });

    Ok(handle)
}

/// Spawn a Libp2p interface and hook it up to this actor
#[instrument(name = "libp2p", skip_all)]
pub fn setup_net(
    topic: &str,
    bus: BusHandle,
    eventstore: impl Into<Recipient<EventStoreQueryBy<TsAgg>>>,
    interface: impl NetInterface,
) -> Result<()> {
    // NOTE: Pass the unbuffered rx to SyncManager as it must operate before live events are
    // processed
    let _net_sync = NetSyncManager::setup(
        &bus,
        &interface.tx(),
        &Arc::new(interface.rx()),
        eventstore.into(),
    );

    // Buffer all incoming events until SyncEnded
    let rx = Arc::new(NetEventBuffer::setup(&bus, &interface.rx()));
    let tx = interface.tx();

    let runner = run_once::<EffectsEnabled>({
        let bus = bus.clone();
        let rx = rx.clone();
        let topic = topic.to_owned();
        let tx = tx.clone();
        move |_| {
            NetEventTranslator::setup(&bus, &tx, &rx, &topic);
            DocumentPublisher::setup(&bus, &tx, &rx, &topic);
            Ok(())
        }
    });

    bus.subscribe(e3_events::EventType::EffectsEnabled, runner.recipient());

    Ok(())
}
