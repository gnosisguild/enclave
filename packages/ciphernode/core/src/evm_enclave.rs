use crate::{
    events,
    evm_listener::{AddEventHandler, ContractEvent, StartListening},
    evm_manager::{AddListener, EvmContractManager},
    EnclaveEvent, EventBus,
};
use actix::Addr;
use alloy::{
    hex,
    primitives::{Address, Bytes},
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

impl TryFrom<&E3Requested> for events::E3Requested {
    type Error = anyhow::Error;
    fn try_from(value: &E3Requested) -> Result<Self, Self::Error> {
        let program_params = value.e3.e3ProgramParams.to_vec();
        println!("received: {}", hex::encode(&program_params));

        let decoded =
            decode_e3_params(&program_params).context("Failed to ABI decode program_params")?;
        Ok(events::E3Requested {
            params: decoded.0.into(),
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

pub fn decode_e3_params(bytes: &[u8]) -> Result<(Vec<u8>, String)> {
    let decoded: (Bytes, Address) = SolValue::abi_decode_params(bytes, true)?;
    Ok((decoded.0.into(), decoded.1.to_string()))
}

pub fn encode_e3_params(params: &[u8], input_validator: Address) -> Vec<u8> {
    (params, input_validator).abi_encode_params()
}

#[cfg(test)]
mod tests {
    use crate::encode_bfv_params;

    use super::{decode_e3_params, encode_e3_params};
    use alloy::{hex, primitives::address};
    use anyhow::*;
    use fhe::bfv::BfvParameters;
    use fhe_traits::Deserialize;

    #[test]
    fn test_evm_decode() -> Result<()> {
        let params_encoded = encode_bfv_params(vec![0x3FFFFFFF000001], 2048, 1032193);

        let add = address!("8A791620dd6260079BF849Dc5567aDC3F2FdC318");
        let encoded = hex::encode(&encode_e3_params(&params_encoded, add));
        assert_eq!(encoded, "00000000000000000000000000000000000000000000000000000000000000400000000000000000000000008a791620dd6260079bf849dc5567adc3f2fdc31800000000000000000000000000000000000000000000000000000000000000130880101208818080f8ffffff1f1881803f200a00000000000000000000000000");
        let input: Vec<u8> = hex::decode(&encoded)?;
        let (de_params, de_address) = decode_e3_params(&input)?;
        let params_assemble = BfvParameters::try_deserialize(&de_params)?;
        assert_eq!(params_assemble.degree(), 2048);
        assert_eq!(de_address, "0x8A791620dd6260079BF849Dc5567aDC3F2FdC318");
        Ok(())
    }
}
