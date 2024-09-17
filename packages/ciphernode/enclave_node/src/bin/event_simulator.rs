use std::sync::Arc;

use alloy_primitives::{address, Address};
use base64::{engine::general_purpose, Engine as _};
use enclave_core::{
    setup_crp_params, Actor, CommitteeRequested, E3id, EnclaveEvent, EventBus, P2p, ParamsWithCrp,
};
use enclave_core::{CiphernodeAdded, CiphertextOutputPublished};
use fhe::bfv::{Encoding, Plaintext, PublicKey};
use fhe_traits::{DeserializeParametrized, FheEncoder};
use rand::SeedableRng;
use rand_chacha::rand_core::OsRng;
use std::fs;
use tokio::{
    self,
    io::{self, AsyncBufReadExt, BufReader},
};

const ADDRS: [Address; 3] = [
    address!("Cc6c693FDB68f0DB58172639CDEa33FF488cf0a5"),
    address!("75437e59cAC691C0624e089554834619dc49B944"),
    address!("e3092f4A2B59234a557aa2dE5D97314D4E969764"),
];

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rng = Arc::new(std::sync::Mutex::new(
        rand_chacha::ChaCha20Rng::from_rng(OsRng).expect("Failed to create RNG"),
    ));

    let bus = EventBus::new(true).start();
    let (_, t1) = P2p::spawn_libp2p(bus.clone())?;
    let mut stdin = BufReader::new(io::stdin()).lines();
    let ParamsWithCrp {
        moduli,
        degree,
        plaintext_modulus,
        crp_bytes,
        params,
        ..
    } = setup_crp_params(&[0x3FFFFFFF000001], 2048, 1032193, rng.clone());
    let t2 = tokio::spawn(async move {
        let mut id: u32 = 1000;
        while let Ok(Some(line)) = stdin.next_line().await {
            let parts: Vec<&str> = line.split_whitespace().collect();
            match parts.as_slice() {
                ["reg", "1"] => bus.do_send(EnclaveEvent::from(CiphernodeAdded {
                    address: ADDRS[0],
                    index: 0,
                    num_nodes: 1,
                })),
                ["reg", "2"] => bus.do_send(EnclaveEvent::from(CiphernodeAdded {
                    address: ADDRS[1],
                    index: 1,
                    num_nodes: 2,
                })),
                ["reg", "3"] => bus.do_send(EnclaveEvent::from(CiphernodeAdded {
                    address: ADDRS[2],
                    index: 2,
                    num_nodes: 3,
                })),
                ["com"] => {
                    id += 1;
                    println!("Requesting comittee: {}", id);
                    bus.do_send(EnclaveEvent::from(CommitteeRequested {
                        e3_id: E3id::from(id),
                        nodecount: 3,
                        sortition_seed: 100,
                        moduli: moduli.clone(),
                        plaintext_modulus,
                        degree,
                        crp: crp_bytes.clone(),
                    }));
                }
                ["cipher"] => {
                    println!("Loading from ./scripts/encrypted.b64...");
                    let encoded_string = fs::read_to_string("scripts/encrypted.b64").unwrap();

                    let decoded_bytes: Vec<u8> = general_purpose::STANDARD
                        .decode(encoded_string.trim())
                        .unwrap();

                    bus.do_send(EnclaveEvent::from(CiphertextOutputPublished {
                        e3_id: E3id::from(id),
                        ciphertext_output: decoded_bytes,
                    }))
                }
                _ => println!("Unknown command"),
            }
        }
    });

    let _ = tokio::join!(t1, t2);
    Ok(())
}
