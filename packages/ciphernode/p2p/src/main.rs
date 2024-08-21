use futures::stream::StreamExt;
use libp2p::{gossipsub, mdns, noise, swarm::NetworkBehaviour, swarm::SwarmEvent, tcp, yamux, identity, Swarm};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::{io, io::AsyncBufReadExt, select};
use tracing_subscriber::EnvFilter;
use bfv::EnclaveBFV;

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub struct EnclaveRouter {
	pub identity: Option<identity::Keypair>,
	pub gossipsub_config: gossipsub::Config,
	pub swarm: Option<libp2p::Swarm<MyBehaviour>>,
	pub topic: Option<gossipsub::IdentTopic>,
}

impl EnclaveRouter {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };

        // TODO: Allow for config inputs to new()
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(message_id_fn)
            .build()
            .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?;

        Ok(Self { identity: None, gossipsub_config, swarm: None, topic: None })
    }

    pub fn with_identiy(&mut self, keypair: identity::Keypair) {
    	self.identity = Some(keypair);
    }

    pub fn connect_swarm(&mut self, discovery_type: String) -> Result<(&Self), Box<dyn Error>> {
    	match discovery_type.as_str() {
    		"mdns" => {
			    let _ = tracing_subscriber::fmt()
			        .with_env_filter(EnvFilter::from_default_env())
			        .try_init();

			    // TODO: Use key if assigned already
			    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
			        .with_tokio()
			        .with_tcp(
			            tcp::Config::default(),
			            noise::Config::new,
			            yamux::Config::default,
			        )?
			        .with_quic()
			        .with_behaviour(|key| {
			            let gossipsub = gossipsub::Behaviour::new(
			                gossipsub::MessageAuthenticity::Signed(key.clone()),
			                self.gossipsub_config.clone(),
			            )?;

			            let mdns =
			                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
			            Ok(MyBehaviour { gossipsub, mdns })
			        })?
			        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
			        .build();
			    self.swarm = Some(swarm);
		    },
    		_ => println!("Defaulting to MDNS discovery"),
    	}
    	Ok((self))
    }

    pub fn join_topic(&mut self, topic_name: &str) -> Result<(&Self), Box<dyn Error>> {
    	let topic = gossipsub::IdentTopic::new(topic_name);
    	self.topic = Some(topic.clone());
    	self.swarm.as_mut().unwrap().behaviour_mut().gossipsub.subscribe(&topic)?;
    	Ok((self))
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
	    // Read full lines from stdin
	    let mut stdin = io::BufReader::new(io::stdin()).lines();

	    self.swarm.as_mut().unwrap().listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
	    self.swarm.as_mut().unwrap().listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
	    loop {
	        select! {
	            Ok(Some(line)) = stdin.next_line() => {
	            	println!("Generating Public Key Share");
	            	let node_bfv = bfv::EnclaveBFV::new("test".to_string());
	                if let Err(e) = self.swarm.as_mut().unwrap()
	                    .behaviour_mut().gossipsub
	                    .publish(self.topic.as_mut().unwrap().clone(), line.as_bytes()) {
	                    println!("Publish error: {e:?}");
	                }
	            }
	            event = self.swarm.as_mut().unwrap().select_next_some() => match event {
	                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
	                    for (peer_id, _multiaddr) in list {
	                        println!("mDNS discovered a new peer: {peer_id}");
	                        self.swarm.as_mut().unwrap().behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
	                    }
	                },
	                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
	                    for (peer_id, _multiaddr) in list {
	                        println!("mDNS discover peer has expired: {peer_id}");
	                        self.swarm.as_mut().unwrap().behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
	                    }
	                },
	                SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
	                    propagation_source: peer_id,
	                    message_id: id,
	                    message,
	                })) => {
	                	println!(
	                        "Got message: '{}' with id: {id} from peer: {peer_id}",
	                        String::from_utf8_lossy(&message.data),
	                    );
	                    let node_bfv = bfv::EnclaveBFV::new("test".to_string());
	            	},
	                SwarmEvent::NewListenAddr { address, .. } => {
	                    println!("Local node is listening on {address}");
	                }
	                _ => {}
	            }
	        }
	    }

	    Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let mut p2p = EnclaveRouter::new()?;
    p2p.connect_swarm("mdns".to_string())?;
    p2p.join_topic("enclave-keygen-01")?;
    p2p.start().await?;

    println!("Hello, cipher world!");
    Ok(())
}
