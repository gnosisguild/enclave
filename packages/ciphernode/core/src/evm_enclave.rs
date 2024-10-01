use crate::{
    events,
    evm_listener::{AddEventHandler, ContractEvent, StartListening},
    evm_manager::{AddListener, EvmContractManager},
    EnclaveEvent, EventBus,
};
use actix::Addr;
use alloy::{
    primitives::{Address},
    sol,
    sol_types::SolValue,
};
use anyhow::{Context, Result};

sol! {
    #[derive(Debug)]
    struct E3 {
        uint256 seed;
        uint32[2] threshold;
        uint256[2] startWindow;
        uint256 duration;
        uint256 expiration;
        bytes32 encryptionSchemeId;
        address e3Program;
        bytes e3ProgramParams;
        address inputValidator;
        address decryptionVerifier;
        bytes32 committeePublicKey;
        bytes32 ciphertextOutput;
        bytes plaintextOutput;
    }

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

impl TryFrom<&E3Requested> for events::E3Requested {
    type Error = anyhow::Error;
    fn try_from(value: &E3Requested) -> Result<Self, Self::Error> {
        let program_params = value.e3.e3ProgramParams.to_vec();
        Ok(events::E3Requested {
            params: program_params.into(),
            threshold_m: value.e3.threshold[0] as usize,
            seed: value.e3.seed.into(),
            e3_id: value.e3Id.to_string().into(),
        })
    }
}

impl From<CiphertextOutputPublished> for events::CiphertextOutputPublished {
    fn from(value: CiphertextOutputPublished) -> Self {
        events::CiphertextOutputPublished {
            e3_id: value.e3Id.to_string().into(),
            ciphertext_output: value.ciphertextOutput.to_vec(),
        }
    }
}

impl ContractEvent for E3Requested {
    fn process(&self, bus: Addr<EventBus>) -> Result<()> {
        let data: events::E3Requested = self.try_into()?;

        bus.do_send(EnclaveEvent::from(data));
        Ok(())
    }
}

impl ContractEvent for CiphertextOutputPublished {
    fn process(&self, bus: Addr<EventBus>) -> Result<()> {
        let data: events::CiphertextOutputPublished = self.clone().into();
        bus.do_send(EnclaveEvent::from(data));
        Ok(())
    }
}

pub async fn connect_evm_enclave(bus: Addr<EventBus>, rpc_url: &str, contract_address: Address) {
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
        .send(AddEventHandler::<CiphertextOutputPublished>::new())
        .await
        .unwrap();
    evm_listener.do_send(StartListening);

    println!("Evm is listening to {}", contract_address);
}

