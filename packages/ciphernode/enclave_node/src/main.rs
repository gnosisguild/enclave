use std::error::Error;

use alloy::primitives::address;
use bfv::EnclaveBFV;
use p2p::EnclaveRouter;
use sortition::DistanceSortition;
use tokio::{
    self,
    io::{self, AsyncBufReadExt, BufReader},
};

const OWO: &str = r#"
      ___           ___           ___                         ___                         ___     
     /\__\         /\  \         /\__\                       /\  \          ___          /\__\    
    /:/ _/_        \:\  \       /:/  /                      /::\  \        /\  \        /:/ _/_   
   /:/ /\__\        \:\  \     /:/  /                      /:/\:\  \       \:\  \      /:/ /\__\  
  /:/ /:/ _/_   _____\:\  \   /:/  /  ___   ___     ___   /:/ /::\  \       \:\  \    /:/ /:/ _/_ 
 /:/_/:/ /\__\ /::::::::\__\ /:/__/  /\__\ /\  \   /\__\ /:/_/:/\:\__\  ___  \:\__\  /:/_/:/ /\__\
 \:\/:/ /:/  / \:\~~\~~\/__/ \:\  \ /:/  / \:\  \ /:/  / \:\/:/  \/__/ /\  \ |:|  |  \:\/:/ /:/  /
  \::/_/:/  /   \:\  \        \:\  /:/  /   \:\  /:/  /   \::/__/      \:\  \|:|  |   \::/_/:/  / 
   \:\/:/  /     \:\  \        \:\/:/  /     \:\/:/  /     \:\  \       \:\__|:|__|    \:\/:/  /  
    \::/  /       \:\__\        \::/  /       \::/  /       \:\__\       \::::/__/      \::/  /   
     \/__/         \/__/         \/__/         \/__/         \/__/        ~~~~           \/__/    
                                                                      
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // boot up p2p network

    // boot up ether client

    // start main loop

    //let ether = eth::EtherClient::new("test".to_string());
    println!("\n\n\n\n\n{}", OWO);
    println!("\n\n\n\n");
    println!("Hello, cipher world!");

    let mut committee = DistanceSortition::new(
        12,
        vec![address!("d8da6bf26964af9d7eed9e03e53415d37aa96045")],
        10,
    );
    committee.get_committee();

    let mut new_bfv = EnclaveBFV::new(4096, 4096, vec![0xffffee001, 0xffffc4001, 0x1ffffe0001]);
    let pk_bytes = new_bfv.serialize_pk();
    let param_bytes = new_bfv.serialize_params();
    let crp_bytes = new_bfv.serialize_crp();
    let deserialized_pk = new_bfv.deserialize_pk(pk_bytes, param_bytes, crp_bytes);

    let (mut p2p, tx, mut rx) = EnclaveRouter::new()?;
    p2p.connect_swarm("mdns".to_string())?;
    p2p.join_topic("enclave-keygen-01")?;
    let mut stdin = BufReader::new(io::stdin()).lines();
    tokio::spawn(async move { p2p.start().await });
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            println!("msg: {}", String::from_utf8(msg).unwrap());
        }
    });
    loop {
        if let Ok(Some(line)) = stdin.next_line().await {
            tx.send(line.as_bytes().to_vec().clone()).await.unwrap();
        }
    }
}
