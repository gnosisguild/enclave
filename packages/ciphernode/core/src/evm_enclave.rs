use std::sync::Arc;

use crate::{
    events,
    evm_listener::{AddEventHandler, ContractEvent, StartListening},
    evm_manager::{AddListener, EvmContractManager},
    setup_crp_params, EnclaveEvent, EventBus, ParamsWithCrp,
};
use actix::Addr;
use alloy::{primitives::Address, sol};
use anyhow::Result;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

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
        bytes committeePublicKey;
        bytes32 ciphertextOutput;
        bytes32 plaintextOutput;
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

impl From<E3Requested> for events::E3Requested {
    fn from(value: E3Requested) -> Self {
        let _params_bytes = value.e3.e3ProgramParams;
        // TODO: decode params bytes
        // HACK: temp supply canned params:
        // this is temporary parse this from params_bytes above
        // We will parse the ABI encoded bytes and extract params
        let ParamsWithCrp {
            moduli,
            degree,
            plaintext_modulus,
            crp_bytes,
            ..
        } = setup_crp_params(
            &[0x3FFFFFFF000001],
            2048,
            1032193,
            // HACK: This is required to be fixed in order to have the same CRP bytes which will be
            // resolved once we are decrypting params from the contract
            Arc::new(std::sync::Mutex::new(ChaCha20Rng::seed_from_u64(42))),
        );
        events::E3Requested {
            moduli,
            plaintext_modulus,
            degree,
            threshold_m: value.e3.threshold[0] as usize,
            crp: crp_bytes,
            // HACK: Following should be [u8;32] and not converted to u64
            seed: value.e3.seed.try_into().unwrap_or_default(), // converting to u64
            e3_id: value.e3Id.to_string().into(),
        }
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
        let data: events::E3Requested = self.clone().into();
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
