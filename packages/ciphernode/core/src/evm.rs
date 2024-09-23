use crate::{
    events, evm_listener::{AddEventHandler, ContractEvent, EvmEventListener, StartListening}, evm_manager::{AddListener, EvmContractManager}, EnclaveEvent, EventBus
};
use actix::{Actor, Addr, Context};
use alloy::{primitives::Address, sol, sol_types::SolEvent};
use anyhow::Result;

sol! {
    #[derive(Debug)]
    struct E3 {
        uint256 seed;
        uint32[2] threshold;
        uint256[2] startWindow;
        uint256 duration;
        uint256 expiration;
        address e3Program;
        bytes e3ProgramParams;
        address inputValidator;
        address decryptionVerifier;
        bytes committeePublicKey;
        bytes ciphertextOutput;
        bytes plaintextOutput;
    }

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

    #[derive(Debug)]
    event E3Requested(
        uint256 e3Id,
        E3 e3,
        address filter,
        address indexed e3Program
    );
}

struct Evm {
    // holding refs to evm contracts for management
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
            .send(AddEventHandler::<E3Requested>::new())
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

impl Actor for Evm {
    type Context = Context<Self>;
}
