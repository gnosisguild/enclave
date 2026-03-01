// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pub mod application;
pub mod ciphernode_system;
pub mod libp2p_mock;
mod plaintext_writer;
mod public_key_writer;
pub mod usecase_helpers;
mod utils;
use actix::prelude::*;
use alloy::primitives::Address;
use anyhow::Result;
use e3_ciphernode_builder::{CiphernodeHandle, EventSystem};
use e3_events::{
    BusHandle, CiphernodeAdded, Enabled, EnclaveEvent, EnclaveEventData, EventBus, EventBusConfig,
    EventContextAccessors, EventPublisher, EventType, HistoryCollector, Seed, Sequenced, Subscribe,
};
use e3_fhe_params::BfvParamSet;
use e3_fhe_params::DEFAULT_BFV_PRESET;
use e3_fhe_params::{build_bfv_params_arc, create_deterministic_crp_from_default_seed};
use e3_net::{DocumentPublisher, NetEventTranslator};
use e3_utils::SharedRng;
use fhe::bfv::{BfvParameters, Ciphertext, Encoding, Plaintext, PublicKey};
use fhe::mbfv::CommonRandomPoly;
use fhe_traits::Serialize;
use fhe_traits::{FheEncoder, FheEncrypter};
use libp2p_mock::Libp2pMock;
pub use plaintext_writer::*;
pub use public_key_writer::*;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::sync::Arc;
pub use utils::*;

pub fn create_shared_rng_from_u64(value: u64) -> Arc<std::sync::Mutex<ChaCha20Rng>> {
    Arc::new(std::sync::Mutex::new(ChaCha20Rng::seed_from_u64(value)))
}

pub fn create_seed_from_u64(value: u64) -> Seed {
    Seed(ChaCha20Rng::seed_from_u64(value).get_seed())
}

pub fn create_rng_from_seed(seed: Seed) -> SharedRng {
    Arc::new(std::sync::Mutex::new(ChaCha20Rng::from_seed(seed.into())))
}

pub fn create_crp_bytes_params(
    moduli: &[u64],
    degree: usize,
    plaintext_modulus: u64,
) -> (Vec<u8>, Arc<BfvParameters>) {
    let params = build_bfv_params_arc(degree, plaintext_modulus, moduli, None);
    let crp = create_deterministic_crp_from_default_seed(&params);

    (crp.to_bytes(), params)
}

pub fn get_common_setup(
    param_set: Option<BfvParamSet>,
) -> Result<(
    BusHandle,
    SharedRng,
    Seed,
    Arc<BfvParameters>,
    CommonRandomPoly,
    Addr<HistoryCollector<EnclaveEvent>>,
    Addr<HistoryCollector<EnclaveEvent>>,
)> {
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
    let errors = HistoryCollector::<EnclaveEvent>::new().start();
    let history = HistoryCollector::<EnclaveEvent>::new().start();
    bus.do_send(Subscribe::new(EventType::All, history.clone().recipient()));
    bus.do_send(Subscribe::new(
        EventType::EnclaveError,
        errors.clone().recipient(),
    ));

    let rng = create_shared_rng_from_u64(42);
    let seed = create_seed_from_u64(123);
    let param_set = param_set.unwrap_or(DEFAULT_BFV_PRESET.into());
    let degree = param_set.degree;
    let plaintext_modulus = param_set.plaintext_modulus;
    let moduli = param_set.moduli;
    let (crp_bytes, params) = create_crp_bytes_params(moduli, degree, plaintext_modulus);
    let crpoly = CommonRandomPoly::deserialize(&crp_bytes.clone(), &params)?;
    let handle = EventSystem::in_mem()
        .with_event_bus(bus)
        .handle()?
        .enable("cn1");
    Ok((handle, rng, seed, params, crpoly, errors, history))
}

/// Actor that pipes events between buses, filtering for broadcastable events
/// and transforming document-publisher events to simulate network receipt
/// (e.g. setting `external: true` on `DecryptionKeyShared`).
struct SimulatedNetPipe {
    dest: BusHandle<Enabled>,
}

impl Actor for SimulatedNetPipe {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent<Sequenced>> for SimulatedNetPipe {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent<Sequenced>, _: &mut Self::Context) -> Self::Result {
        let should_forward = NetEventTranslator::is_forwardable_event(&msg)
            || DocumentPublisher::is_document_publisher_event(&msg);

        if should_forward {
            let source = msg.source();
            let (mut data, ts) = msg.split();

            // Simulate network receive: in production, DocumentPublisher
            // sets external=true when reconstructing events from the network.
            if let EnclaveEventData::DecryptionKeyShared(ref mut dks) = data {
                dks.external = true;
            }

            let _ = self.dest.publish_from_remote(data, ts, None, source);
        }
    }
}

/// Simulate libp2p by taking output events on each local bus and filter for !is_local_only() and forward remaining events back to the event bus
/// deduplication will remove previously seen events.
/// This sets up a set of cyphernodes without libp2p.
/// The way it works is that it feeds back all events from
/// all nodes filteres by whether they are broadcastible or not
/// ```txt
///
///                    ┌─────┐
///                    │ BUS │
///                    └─────┘
///                       │
///          ┌────────────┼────────────┐
///          │            │            │
///          ▼            ▼            ▼
///       ┌────┐       ┌────┐       ┌────┐
///       │ B1 │       │ B2 │       │ B3 │◀──┐
///       └────┘       └────┘       └────┘   │
///          │            │            │     │
///          │            │            │     │
///          └────────────┼────────────┘     │
///                       │                  │
///                       ▼                  │
///                    ┌─────┐               │
///                    │ FIL │───────────────┘
///                    └─────┘
/// ```
pub async fn simulate_libp2p_net(nodes: &[CiphernodeHandle]) {
    println!("MOCK: simulate_libp2p_net");
    let mock = Libp2pMock::new();
    for node in nodes.iter() {
        let interface = node
            .net_simulate_adaptor
            .clone()
            .expect("net_simulate_adaptor must be set for simulated nodes");
        mock.add_node(node.peer_id, interface).await;
    }
}

// fn pipe(src: NetInterfaceInvertedHandle, dest: NetInterfaceInvertedHandle) {
//     let src_event_tx = src.event_tx();
//     let dest_event_tx = dest.event_tx();
//     let mut src_cmd_rx = src.cmd_rx();
//
//     tokio::spawn(async move {
//         let mut store: HashMap<ContentHash, ArcBytes> = HashMap::new();
//
//         loop {
//             match src_cmd_rx.recv().await {
//                 Ok(NetCommand::GossipPublish {
//                     data,
//                     correlation_id,
//                     ..
//                 }) => {
//                     if let Err(e) = dest_event_tx.send(NetEvent::GossipData(data)) {
//                         error!("pipe: failed to forward GossipData to dest: {e}");
//                     }
//
//                     let message_id = MessageId::new(&format!("{correlation_id:?}").into_bytes());
//                     if let Err(e) = src_event_tx.send(NetEvent::GossipPublished {
//                         correlation_id,
//                         message_id,
//                     }) {
//                         error!("pipe: failed to send GossipPublished to src: {e}");
//                     }
//                 }
//                 Ok(NetCommand::DhtPutRecord {
//                     correlation_id,
//                     key,
//                     value,
//                     ..
//                 }) => {
//                     store.insert(key.clone(), value.clone());
//
//                     if let Err(e) = dest_event_tx.send(NetEvent::DhtGetRecordSucceeded {
//                         key: key.clone(),
//                         correlation_id,
//                         value,
//                     }) {
//                         error!("pipe: failed to forward DhtGetRecordSucceeded to dest: {e}");
//                     }
//
//                     if let Err(e) = src_event_tx.send(NetEvent::DhtPutRecordSucceeded {
//                         key,
//                         correlation_id,
//                     }) {
//                         error!("pipe: failed to send DhtPutRecordSucceeded to src: {e}");
//                     }
//                 }
//                 Ok(NetCommand::DhtGetRecord {
//                     correlation_id,
//                     key,
//                 }) => {
//                     if let Some(value) = store.get(&key).cloned() {
//                         if let Err(e) = src_event_tx.send(NetEvent::DhtGetRecordSucceeded {
//                             key,
//                             correlation_id,
//                             value,
//                         }) {
//                             error!("pipe: failed to send DhtGetRecordSucceeded to src: {e}");
//                         }
//                     } else {
//                         if let Err(e) = src_event_tx.send(NetEvent::DhtGetRecordError {
//                             correlation_id,
//                             error: GetRecordError::NotFound {
//                                 key: RecordKey::new(&key.into_inner()),
//                                 closest_peers: vec![],
//                             },
//                         }) {
//                             error!("pipe: failed to send DhtGetRecordError to src: {e}");
//                         }
//                     }
//                 }
//                 Err(broadcast::error::RecvError::Lagged(n)) => {
//                     warn!("pipe: src cmd receiver lagged by {n} messages");
//                     continue;
//                 }
//                 Err(_) => break,
//                 _ => continue,
//             }
//         }
//     });
// }

/// Creates test eth addresses
/// NOTE: THESE ARE NOT ACTUAL ADDRESSES JUST RANDOM DATA
pub fn create_random_eth_addrs(how_many: u32) -> Vec<String> {
    (0..how_many)
        .map(|_| Address::from_slice(&rand::thread_rng().gen::<[u8; 20]>()).to_string())
        .collect()
}

/// Test helper to add addresses to the committee by creating events on the event bus
#[derive(Clone, Debug)]
pub struct AddToCommittee {
    bus: BusHandle,
    count: usize,
    chain_id: u64,
}

impl AddToCommittee {
    pub fn new(bus: &BusHandle, chain_id: u64) -> Self {
        Self {
            bus: bus.clone(),
            chain_id,
            count: 0,
        }
    }
    pub async fn add(&mut self, address: &str) -> Result<EnclaveEventData> {
        let evt = CiphernodeAdded {
            chain_id: self.chain_id,
            address: address.to_owned(),
            index: self.count,
            num_nodes: self.count + 1,
        };

        self.count += 1;

        self.bus.publish_without_context(evt.clone())?;

        Ok(evt.into())
    }
}

pub fn encrypt_ciphertext(
    params: &Arc<BfvParameters>,
    pubkey: PublicKey,
    raw_plaintext: Vec<Vec<u64>>,
) -> Result<(Vec<Ciphertext>, Vec<Plaintext>)> {
    let mut rng = ChaCha20Rng::seed_from_u64(42);
    let plaintext: Vec<_> = raw_plaintext
        .into_iter()
        .map(|raw| Ok(Plaintext::try_encode(&raw, Encoding::poly(), &params)?))
        .collect::<Result<_>>()?;

    let ciphertext = plaintext
        .iter()
        .map(|pt| {
            pubkey
                .try_encrypt(&pt, &mut rng)
                .map_err(|e| anyhow::anyhow!("{e}"))
        })
        .collect::<Result<Vec<Ciphertext>>>()?;
    Ok((ciphertext, plaintext))
}
