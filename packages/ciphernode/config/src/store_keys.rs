use enclave_core::E3id;

pub struct StoreKeys;

impl StoreKeys {
    pub fn keyshare(e3_id: &E3id) -> String {
        format!("//keyshare/{e3_id}")
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

    pub fn enclave_sol_reader(chain_id: u64) -> String {
        format!("//evm_readers/enclave/{chain_id}")
    }

    pub fn ciphernode_registry_reader(chain_id: u64) -> String {
        format!("//evm_readers/ciphernode_registry/{chain_id}")
    }
}
