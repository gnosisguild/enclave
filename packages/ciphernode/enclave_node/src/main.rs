use std::error::Error;

use p2p::EnclaveRouter;
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
