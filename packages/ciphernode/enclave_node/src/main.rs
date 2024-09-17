use std::error::Error;
use std::{str};

use p2p::{EnclaveRouter, P2PMessage};
use bfv::EnclaveBFV;
use sortition::DistanceSortition;
use tokio::{
    self,
    io::{self, AsyncBufReadExt, BufReader},
};

use alloy_primitives::{address};

use log::Level;

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

fn handle_p2p_msg(msg: Vec<u8>) {
    let msg_out_str = str::from_utf8(&msg).unwrap();
    let msg_out_struct: P2PMessage = serde_json::from_str(&msg_out_str).unwrap();
    println!("msg_topic: {}", msg_out_struct.topic);
    println!("msg_type: {}", msg_out_struct.msg_type);
    println!("msg: {}", String::from_utf8(msg_out_struct.data).unwrap());
}

async fn start_p2p() -> Result<(), Box<dyn Error>> {
    log::info!("Connecting to swarm");
    let (mut p2p, p2p_tx, mut p2p_rx) = EnclaveRouter::new()?;
    p2p.connect_swarm("mdns".to_string())?;
    p2p.join_topic("enclave-testnet")?;
    let mut stdin = BufReader::new(io::stdin()).lines();
    tokio::spawn(async move { p2p.start().await });
    tokio::spawn(async move {
        while let Some(msg) = p2p_rx.recv().await {
            handle_p2p_msg(msg);
        }
    });
    loop {
        if let Ok(Some(line)) = stdin.next_line().await {
            let msg_formatted = P2PMessage {
                topic: "enclave-testnet".to_string(),
                msg_type: "join_main_channel".to_string(),
                data: line.into_bytes(),
            };
            let msg_str = serde_json::to_string(&msg_formatted).unwrap();
            let msg_bytes = msg_str.into_bytes();
            p2p_tx.send(msg_bytes.clone()).await.unwrap();
        }
    }
}

async fn start_eth_listener() {
    log::info!("Listening on E3 Contract");
}

async fn run() {
    tokio::join!(
        start_p2p(),
        start_eth_listener(),
    );
}


fn main() -> Result<(), Box<dyn Error>> {
    // boot up p2p network

    // boot up ether client

    // start main loop

    //let ether = eth::EtherClient::new("test".to_string());
    println!("\n\n\n\n\n{}", OWO);
    println!("\n\n\n\n");

    simple_logger::init_with_level(Level::Info).unwrap();

    let main_rt = tokio::runtime::Runtime::new().unwrap();
    let future = run();
    main_rt.block_on(future);

    let mut committee = DistanceSortition::new(12, vec![address!("d8da6bf26964af9d7eed9e03e53415d37aa96045")], 1);
    committee.get_committee();

    let mut new_bfv = EnclaveBFV::new(4096, 4096, vec![0xffffee001, 0xffffc4001, 0x1ffffe0001]);
    let pk_bytes = new_bfv.serialize_pk();
    let param_bytes = new_bfv.serialize_params();
    let crp_bytes = new_bfv.serialize_crp();
    let deserialized_pk = new_bfv.deserialize_pk(pk_bytes, param_bytes, crp_bytes);

    Ok(())
}
