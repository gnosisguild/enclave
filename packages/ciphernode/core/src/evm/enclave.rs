use actix::Addr;
use alloy::{primitives::Address, sol};
use anyhow::Result;

use crate::{
    enclave_core::{self, EnclaveEvent, EventBus},
    evm::{AddEventHandler, AddListener, EvmContractManager, StartListening},
};

use super::listener::ContractEvent;

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

impl TryFrom<&E3Requested> for enclave_core::E3Requested {
    type Error = anyhow::Error;
    fn try_from(value: &E3Requested) -> Result<Self, Self::Error> {
        let program_params = value.e3.e3ProgramParams.to_vec();
        Ok(enclave_core::E3Requested {
            params: program_params.into(),
            threshold_m: value.e3.threshold[0] as usize,
            seed: value.e3.seed.into(),
            e3_id: value.e3Id.to_string().into(),
        })
    }
}

impl From<CiphertextOutputPublished> for enclave_core::CiphertextOutputPublished {
    fn from(value: CiphertextOutputPublished) -> Self {
        enclave_core::CiphertextOutputPublished {
            e3_id: value.e3Id.to_string().into(),
            ciphertext_output: value.ciphertextOutput.to_vec(),
        }
    }
}

impl ContractEvent for E3Requested {
    fn process(&self, bus: Addr<EventBus>) -> Result<()> {
        let data: enclave_core::E3Requested = self.try_into()?;

        bus.do_send(EnclaveEvent::from(data));
        Ok(())
    }
}

impl ContractEvent for CiphertextOutputPublished {
    fn process(&self, bus: Addr<EventBus>) -> Result<()> {
        let data: enclave_core::CiphertextOutputPublished = self.clone().into();
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
