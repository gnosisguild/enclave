use crate::{events::{NetworkPeerCommand, NetworkPeerEvent}, NetworkPeer};
use actix::prelude::*;
use anyhow::{Context as AnyhowContext, Result};
use libp2p::{
    multiaddr::Protocol,
    swarm::{dial_opts::DialOpts, ConnectionId, DialError},
    Multiaddr,
};
use std::{
    collections::HashMap,
    net::ToSocketAddrs,
    sync::Arc,
    time::Duration,
};
use tracing::{info, warn};

const BACKOFF_DELAY: u64 = 500;
const BACKOFF_MAX_RETRIES: u32 = 10;
const CONNECTION_TIMEOUT: u64 = 60;

#[derive(Message)]
#[rtype(result = "Result<()>")]
pub struct DialPeer(pub String);

#[derive(Message)]
#[rtype(result = "Result<()>")]
pub struct DialPeers(pub Vec<String>);

#[derive(Message)]
#[rtype(result = "Result<()>")]
pub struct SetNetworkPeer(pub Addr<NetworkPeer>);

#[derive(Message)]
#[rtype(result = "()")]
pub struct ConnectionResult {
    pub connection_id: ConnectionId,
    pub result: Result<(), Arc<DialError>>,
}

#[derive(Clone)]
struct PendingConnection {
    addr: String,
    attempt: u32,
    delay_ms: u64,
}

#[derive(Message)]
#[rtype(result = "()")]
struct RetryDial {
    addr: String,
    attempt: u32,
    delay_ms: u64,
}
#[derive(Clone)]
pub struct DialerActor {
    network_peer: Option<Addr<NetworkPeer>>,
    pending_connections: HashMap<ConnectionId, PendingConnection>,
}

impl DialerActor {
    pub fn new() -> Self {
        Self {
            network_peer: None,
            pending_connections: HashMap::new(),
        }
    }

    fn attempt_dial(
        &mut self,
        addr: String,
        attempt: u32,
        delay_ms: u64,
        _ctx: &mut Context<Self>,
    ) -> Option<ConnectionId> {
        info!("Attempt {}/{} for {}", attempt, BACKOFF_MAX_RETRIES, addr);

        match addr.parse::<Multiaddr>() {
            Ok(multi) => {
                let resolved_multiaddr = self.get_resolved_multiaddr(&multi).unwrap();
                let opts: DialOpts = resolved_multiaddr.into();
                let connection_id = opts.connection_id();

                if let Some(network_peer) = &self.network_peer {
                    match network_peer.try_send(NetworkPeerCommand::Dial(opts)) {
                        Ok(_) => {
                            info!("Dialing {} with connection {}", addr, connection_id);
                            self.pending_connections.insert(
                                connection_id,
                                PendingConnection {
                                    addr,
                                    attempt,
                                    delay_ms,
                                },
                            );
                            Some(connection_id)
                        }
                        Err(e) => {
                            warn!("Failed to initiate dial: {}", e);
                            None
                        }
                    }
                } else {
                    warn!("No network peer set for dialing {}", addr);
                    None
                }
            }
            Err(e) => {
                warn!("Invalid multiaddr {}: {}", addr, e);
                None
            }
        }
    }

    fn schedule_retry(&self, addr: String, attempt: u32, delay_ms: u64, ctx: &mut Context<Self>) {
        if attempt < BACKOFF_MAX_RETRIES {
            ctx.address().do_send(RetryDial {
                addr,
                attempt: attempt + 1,
                delay_ms: delay_ms * 2,
            });
        } else {
            warn!("Max retries reached for {}", addr);
        }
    }

    // -----------------------------
    // DNS resolution logic
    // -----------------------------

    fn get_resolved_multiaddr(&self, value: &Multiaddr) -> Result<Multiaddr> {
        if let Some(domain) = self.extract_dns_host(value) {
            let ip = self.resolve_ipv4(&domain)?;
            self.dns_to_ip_addr(value, &ip)
        } else {
            Ok(value.clone())
        }
    }

    fn extract_dns_host(&self, addr: &Multiaddr) -> Option<String> {
        for proto in addr.iter() {
            match proto {
                Protocol::Dns4(hostname) | Protocol::Dns6(hostname) => {
                    return Some(hostname.to_string())
                }
                _ => continue,
            }
        }
        None
    }

    fn dns_to_ip_addr(&self, original: &Multiaddr, ip_str: &str) -> Result<Multiaddr> {
        let ip = ip_str.parse()?;
        let mut new_addr = Multiaddr::empty();
        let mut skip_next = false;

        for proto in original.iter() {
            if skip_next {
                skip_next = false;
                continue;
            }

            match proto {
                Protocol::Dns4(_) | Protocol::Dns6(_) => {
                    new_addr.push(Protocol::Ip4(ip));
                    skip_next = false;
                }
                _ => new_addr.push(proto),
            }
        }

        Ok(new_addr)
    }

    fn resolve_ipv4(&self, domain: &str) -> Result<String> {
        let addr = format!("{}:0", domain)
            .to_socket_addrs()?
            .find(|addr| addr.ip().is_ipv4())
            .context("no IPv4 addresses found")?;
        Ok(addr.ip().to_string())
    }
}

impl Actor for DialerActor {
    type Context = Context<Self>;
}

impl Handler<SetNetworkPeer> for DialerActor {
    type Result = Result<()>;

    fn handle(&mut self, msg: SetNetworkPeer, _: &mut Context<Self>) -> Self::Result {
        self.network_peer = Some(msg.0);
        Ok(())
    }
}

impl Handler<NetworkPeerEvent> for DialerActor {
    type Result = ();

    fn handle(&mut self, msg: NetworkPeerEvent, ctx: &mut Context<Self>) {
        match msg {
            NetworkPeerEvent::ConnectionEstablished { connection_id } => {
                if let Some(conn) = self.pending_connections.remove(&connection_id) {
                    info!("Connection Established for {}", conn.addr);
                }
            }
            NetworkPeerEvent::DialError {
                error,
                connection_id,
            } => {
                if let Some(conn) = self.pending_connections.remove(&connection_id) {
                    warn!("DialError for {}: {}", conn.addr, error);
                    if !matches!(error.as_ref(), DialError::NoAddresses { .. }) {
                        self.schedule_retry(conn.addr, conn.attempt, conn.delay_ms, ctx);
                    } else {
                        warn!("Permanent failure for {}: {}", conn.addr, error);
                    }
                }
            }
            NetworkPeerEvent::OutgoingConnectionError {
                connection_id,
                error,
            } => {
                if let Some(conn) = self.pending_connections.remove(&connection_id) {
                    warn!("OutgoingConnectionError for {}: {}", conn.addr, error);
                    if !matches!(error.as_ref(), DialError::NoAddresses { .. }) {
                        self.schedule_retry(conn.addr, conn.attempt, conn.delay_ms, ctx);
                    } else {
                        warn!("Permanent failure for {}: {}", conn.addr, error);
                    }
                }
            }
            _ => {}
        }
    }
}

impl Handler<DialPeers> for DialerActor {
    type Result = Result<()>;

    fn handle(&mut self, msg: DialPeers, ctx: &mut Context<Self>) -> Self::Result {
        for addr in msg.0 {
            ctx.address().do_send(RetryDial {
                addr,
                attempt: 1,
                delay_ms: BACKOFF_DELAY,
            });
        }
        Ok(())
    }
}

impl Handler<RetryDial> for DialerActor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: RetryDial, _: &mut Context<Self>) -> Self::Result {
        let RetryDial {
            addr,
            attempt,
            delay_ms,
        } = msg;

        let future = async move {
            if attempt > 1 {
                actix::clock::sleep(Duration::from_millis(delay_ms)).await;
            }
        }
        .into_actor(self);

        Box::pin(future.map(move |_, actor, ctx| {
            if let Some(connection_id) = actor.attempt_dial(addr.clone(), attempt, delay_ms, ctx) {
                ctx.run_later(Duration::from_secs(CONNECTION_TIMEOUT), move |act, ctx| {
                    if let Some(conn) = act.pending_connections.remove(&connection_id) {
                        warn!("Connection attempt timed out for {}", conn.addr);
                        act.schedule_retry(conn.addr, conn.attempt, conn.delay_ms, ctx);
                    }
                });
            }
        }))
    }
}
