// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, Eq, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Contract {
    Full {
        address: String,
        deploy_block: Option<u64>,
    },
    AddressOnly(String),
}

impl Contract {
    pub fn address(&self) -> &String {
        use Contract::*;
        match self {
            Full { address, .. } => address,
            AddressOnly(v) => v,
        }
    }

    pub fn deploy_block(&self) -> Option<u64> {
        use Contract::*;
        match self {
            Full { deploy_block, .. } => deploy_block.clone(),
            AddressOnly(_) => None,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub struct ContractAddresses {
    pub enclave: Contract,
    pub ciphernode_registry: Contract,
    pub bonding_registry: Contract,
    pub e3_program: Option<Contract>,
}
