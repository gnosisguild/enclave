use std::error::Error;
use std::{str, env};
use std::process;
use std::mem::size_of_val;

use p2p::{EnclaveRouter, P2PMessage};
use bfv::EnclaveBFV;
use database::{EnclaveDB, State, Computation};
use sortition::DistanceSortition;
use eth::{EventListener, ContractManager, CommitteeRequestedEvent, ETHEvent, EventType};
use tokio::{
    self,
    io::{self, AsyncBufReadExt, BufReader},
    sync::mpsc::{channel, Receiver, Sender},
};

use alloy_primitives::{address as paddress, FixedBytes};
use alloy::{
    primitives::{Address, address},
    sol,
};

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

fn aggregate_key(db: EnclaveDB, id: String, pubkey_share: Vec<u8>, pk_shares: Vec<Vec<u8>>) {
    println!("got pk data from {:?}", id);
    println!("{:?}", pk_shares.len());
}

async fn send_p2p_msg(
    p2p_tx: Sender<Vec<u8>>,
    topic: String,
    msg_type: String,
    id: String,
    data: Vec<u8>
) {
    let msg_formatted = P2PMessage {
        topic: topic,
        msg_type: msg_type,
        msg_sender: id,
        data: data,
    };
    let msg_str = serde_json::to_string(&msg_formatted).unwrap();
    let msg_bytes = msg_str.into_bytes();
    p2p_tx.send(msg_bytes.clone()).await.unwrap(); 
}

fn handle_p2p_msg(mut db: EnclaveDB, msg: Vec<u8>) {
    let msg_out_str = str::from_utf8(&msg).unwrap();
    let msg_out_struct: P2PMessage = serde_json::from_str(&msg_out_str).unwrap();
    if msg_out_struct.msg_type == "join and share key" {
        log::info!("New key share received, aggregating...");
        // TODO: If all keys shards gathered
        let state = db.get_state(&"state".to_string());
        let mut comp_state = db.get_computation_state(&state.e3Ids.last().unwrap().to_string());
        comp_state.pk_shares.push(msg_out_struct.data.clone());
        aggregate_key(db, msg_out_struct.msg_sender, msg_out_struct.data.clone(), comp_state.pk_shares.clone());
    }
    log::info!("P2P Message Received: Topic {}, Type {}", msg_out_struct.topic, msg_out_struct.msg_type);
    //log::info!("P2P Message Received: Topic {}, Type {}, Data {}", msg_out_struct.topic, msg_out_struct.msg_type, String::from_utf8(msg_out_struct.data).unwrap());
}

async fn handle_eth_event(mut db: EnclaveDB, msg: Vec<u8>, p2p_tx: Sender<Vec<u8>>) {
    log::info!("Received Committee Requested Event");
    let event_out_str = str::from_utf8(&msg).unwrap();
    let event_out_struct: ETHEvent = serde_json::from_str(&event_out_str).unwrap();
    let mut state = db.get_state(&"state".to_string());
    match event_out_struct.event_type {
        EventType::CommitteeRequested => {
            let committee_event = event_out_struct.committee_requested.unwrap();
            log::info!("Committee Request: e3Id {}", committee_event.e3Id);
            let mut committee = DistanceSortition::new(122, state.nodes.clone(), committee_event.threshold[0] as usize);
            let selected = committee.get_committee();

            let e3Id = committee_event.e3Id;

            if selected.iter().any(|node| node.1 == state.id) {
                log::info!("Selected for Committee, Join Gossip Channel");
                // sub to new topic and join
                // generate keyshare
                let mut new_bfv = EnclaveBFV::new(4096, 4096, vec![0xffffee001, 0xffffc4001, 0x1ffffe0001], 1337);
                let mut pk_bytes = new_bfv.serialize_pk();
                let param_bytes = new_bfv.serialize_params();
                let crp_bytes = new_bfv.serialize_crp();
                let mut comp_state = Computation {
                    computation_id: committee_event.e3Id.to_string(),
                    committee: selected,
                    pk_share: pk_bytes.clone(),
                    pk_shares: vec![pk_bytes.clone()],
                    rng_seed: 1337,
                };

                db.insert_computation_state(&e3Id.to_string(), comp_state);

                state.e3Ids.push(e3Id.to_string());
                db.insert_state(&"state".to_string(), state.clone());

                //let deserialized_pk = new_bfv.deserialize_pk(pk_bytes, param_bytes, crp_bytes);
                println!("{:?}", size_of_val(&*pk_bytes));
                //pk_bytes.drain(0..52725);
                //println!("{:?}", size_of_val(&*pk_bytes));
                let msg_formatted = P2PMessage {
                    //topic: committee_event.e3Id.to_string(),
                    topic: "enclave-testnet".to_string(),
                    msg_type: "join and share key".to_string(),
                    msg_sender: state.id.clone(),
                    data: pk_bytes.clone(),
                };
                let msg_str = serde_json::to_string(&msg_formatted).unwrap();
                let msg_bytes = msg_str.into_bytes();
                println!("{:?}", size_of_val(&*msg_bytes));
                p2p_tx.send(msg_bytes.clone()).await.unwrap();
            }
            // log::info!("Committee Selected: Node {}", selected[0].1);
            // log::info!("Committee Selected: Node {}", selected[1].1);
        },
        EventType::CiphernodeAdded => {
            let node_address = event_out_struct.ciphernode_added.unwrap().node;
            log::info!("Ciphernode Added: Address {}", node_address.clone());
            state.nodes.push(node_address.to_string());
            db.insert_state(&"state".to_string(), state);
        }
    }
}

fn get_p2p_router() -> Result<(EnclaveRouter, Sender<Vec<u8>>, Receiver<Vec<u8>>), Box<dyn Error>> {
    let (mut p2p, mut p2p_tx, mut p2p_rx) = EnclaveRouter::new()?;
    Ok((p2p, p2p_tx, p2p_rx))
}

async fn start_p2p(mut db: EnclaveDB, mut p2p: EnclaveRouter, mut p2p_tx: Sender<Vec<u8>>, mut p2p_rx: Receiver<Vec<u8>>) -> Result<(), Box<dyn Error>> {
    log::info!("Connecting to swarm");
    p2p.connect_swarm("mdns".to_string())?;
    p2p.join_topic("enclave-testnet")?;
    log::info!("Joined Topic Enclave-Testnet");
    let mut stdin = BufReader::new(io::stdin()).lines();
    tokio::spawn(async move { p2p.start().await });
    tokio::spawn(async move {
        while let Some(msg) = p2p_rx.recv().await {
            handle_p2p_msg(db.clone(), msg);
        }
    });
    Ok(())
}

async fn start_eth_listener(mut db: EnclaveDB, mut p2p_tx: Sender<Vec<u8>>) {
    log::info!("Listening on E3 Contract");
    let (mut manager, tx_sender, mut tx_receiver) = ContractManager::new("ws://127.0.0.1:8545").await.unwrap();
    let listener = manager.add_listener(address!("959922be3caee4b8cd9a407cc3ac1c251c2007b1"));
    tokio::spawn(async move { 
        listener.listen().await;
    });
    while let Some(msg) = tx_receiver.recv().await {
        handle_eth_event(db.clone(), msg, p2p_tx.clone()).await;
    };
}

async fn run(id: Address) {
    let mut db = EnclaveDB::new();

    let node_address = db.get(&"state".to_string());
    if node_address.is_empty() {
        log::info!("Generating DB State");
        let address_bytes = id.to_string().into_bytes();
        let state = State {
            id: id.to_string(),
            nodes: vec![],
            e3Ids: vec![],
        };
        db.insert_state(&"state".to_string(), state);
    }

    let (p2p, p2p_tx, p2p_rx) = get_p2p_router().unwrap();
    tokio::join!(
        start_p2p(db.clone(), p2p, p2p_tx.clone(), p2p_rx),
        start_eth_listener(db.clone(), p2p_tx),
    );
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("\n\n\n\n\n{}", OWO);
    println!("\n\n\n\n");

    //let args: Vec<String> = env::args().collect();
    let mut node_address = std::env::args().nth(1).expect("no pattern given");
    let without_prefix = node_address.trim_start_matches("0x");
    let n_address = without_prefix.parse::<Address>().unwrap();

    simple_logger::init_with_level(Level::Info).unwrap();
    log::info!("Node Address {}", n_address.clone());

    let main_rt = tokio::runtime::Runtime::new().unwrap();
    let future = run(n_address);
    main_rt.block_on(future);

    Ok(())
}
