// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_events::E3id;

pub struct StoreKeys;

impl StoreKeys {
    pub fn keyshare(e3_id: &E3id) -> String {
        format!("//keyshare/{e3_id}")
    }

    pub fn threshold_keyshare(e3_id: &E3id) -> String {
        format!("//threshold_keyshare/{e3_id}")
    }

    pub fn plaintext(e3_id: &E3id) -> String {
        format!("//plaintext/{e3_id}")
    }

    pub fn publickey(e3_id: &E3id) -> String {
        format!("//publickey/{e3_id}")
    }

    pub fn fhe(e3_id: &E3id) -> String {
        format!("//fhe/{e3_id}")
    }

    pub fn meta(e3_id: &E3id) -> String {
        format!("//meta/{e3_id}")
    }

    pub fn context(e3_id: &E3id) -> String {
        format!("//context/{e3_id}")
    }

    pub fn router() -> String {
        String::from("//router")
    }

    pub fn sortition() -> String {
        String::from("//sortition")
    }

    pub fn eth_private_key() -> String {
        String::from("//eth_private_key")
    }

    pub fn libp2p_keypair() -> String {
        String::from("//libp2p/keypair")
    }

    pub fn enclave_sol_reader(chain_id: u64) -> String {
        format!("//evm_readers/enclave/{chain_id}")
    }

    pub fn ciphernode_registry_reader(chain_id: u64) -> String {
        format!("//evm_readers/ciphernode_registry/{chain_id}")
    }

    pub fn bonding_registry_reader(chain_id: u64) -> String {
        format!("//evm_readers/bonding_registry/{chain_id}")
    }

    pub fn node_state() -> String {
        String::from("//node_state")
    }
}
