#![crate_name = "eth"]
#![crate_type = "lib"]
#![warn(missing_docs, unused_imports)]


pub struct EtherClient {
	pub address: String,
}

impl EtherClient {
    fn new(address: String) -> Self {
        Self { address }
    }
}