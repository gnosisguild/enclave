use alloy::primitives::Address;

use crate::SharedRng;
use rand::Rng;

pub fn rand_eth_addr(rng: &SharedRng) -> String {
    {
        let rnum = &mut rng.lock().unwrap().gen::<[u8; 20]>();
        Address::from_slice(rnum).to_string()
    }
}
