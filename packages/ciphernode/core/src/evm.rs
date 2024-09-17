use actix::{Actor, Addr, Context};
use alloy::{primitives::Address, sol, sol_types::SolEvent};
use rand::{thread_rng, RngCore};

use crate::{
    evm_listener::{AddEventHandler, EvmEventListener, StartListening},
    evm_manager::{AddListener, EvmContractManager},
    E3id, EnclaveEvent, EventBus, SharedRng,
};

sol! {
    #[derive(Debug)]
    event CommitteeRequested(
        uint256 indexed e3Id,
        address filter,
        uint32[2] threshold
    );

    #[derive(Debug)]
    event CiphernodeAdded(
        address indexed node,
        uint256 index,
        uint256 numNodes,
        uint256 size
    );

    #[derive(Debug)]
    event CiphernodeRemoved(
        address indexed node,
        uint256 index,
        uint256 numNodes,
        uint256 size
    );

    #[derive(Debug)]
    event CiphertextOutputPublished(
        uint256 indexed e3Id,
        bytes ciphertextOutput
    );
}

struct Evm {
    evm_manager: Addr<EvmContractManager>,
    evm_listener: Addr<EvmEventListener>,
}

impl Evm {
    pub async fn attach(
        bus: Addr<EventBus>,
        rpc_url: &str,
        contract_address: Address,
    ) -> Addr<Evm> {
        let evm_manager = EvmContractManager::attach(bus.clone(), rpc_url).await;
        let evm_listener = evm_manager
            .send(AddListener { contract_address })
            .await
            .unwrap();

        evm_listener
            .send(AddEventHandler::<CommitteeRequested>::new())
            .await
            .unwrap();

        evm_listener
            .send(AddEventHandler::<CiphernodeAdded>::new())
            .await
            .unwrap();

        evm_listener
            .send(AddEventHandler::<CiphernodeRemoved>::new())
            .await
            .unwrap();

        evm_listener
            .send(AddEventHandler::<CiphertextOutputPublished>::new())
            .await
            .unwrap();
        evm_listener.do_send(StartListening);

        Evm {
            evm_listener,
            evm_manager,
        }
        .start()
    }
}

// // Generic conversion for any type that implements SolEvent
// impl<T> From<&T> for EnclaveEvent
// where
//     T: SolEvent + Send + Sync + 'static,
// {
//     fn from(event: &T) -> Self {
//         // Use type_name to get the name of the event type
//         let type_name = std::any::type_name::<T>();
//         match type_name {
//             "CommitteeRequested" => From::from(event as &CommitteeRequested),
//             "CiphernodeAdded" => From::from(event as &CiphernodeAdded),
//             "CiphernodeRemoved" => From::from(event as &CiphernodeRemoved),
//             "CiphertextOutputPublished" => From::from(event as &CiphertextOutputPublished),
//             _ => panic!("Unsupported event type: {}", type_name),
//         }
//     }
// }
//
// impl From<&CommitteeRequested> for EnclaveEvent {
//     fn from(value: CommitteeRequested) -> Self {
//         EnclaveEvent::from(crate::events::CommitteeRequested {
//             e3_id: E3id::from(value.e3Id),
//             nodecount: value.threshold[1] as usize,
//             sortition_seed: thread_rng().next_u64(),
//             // HACK: set params here for event
//             // TODO: pass params with committee requested event
//             crp: Com,
//         })
//     }
// }
//
impl Actor for Evm {
    type Context = Context<Self>;
}
