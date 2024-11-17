use std::time::Duration;

use libp2p::StreamProtocol;

pub const TICK_INTERVAL: Duration = Duration::from_secs(15);
pub const KADEMLIA_PROTOCOL_NAME: StreamProtocol = StreamProtocol::new("/enclave/kad/1.0.0");
pub const PORT_QUIC: u16 = 9091;
pub const LOCAL_KEY_PATH: &str = "./local_key";
pub const LOCAL_CERT_PATH: &str = "./cert.pem";
pub const GOSSIPSUB_PEER_DISCOVERY: &str = "enclave-keygen-peer-discovery";
pub const BOOTSTRAP_NODES: [&str; 0] = [
  // TODO: Add bootstrap nodes
];
