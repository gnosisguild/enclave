use crate::events::{NetworkPeerCommand, NetworkPeerEvent};
use actix::prelude::*;
use anyhow::{Context as AnyhowContext, Result};
use events::{EventBus, Subscribe};
use libp2p::{
    multiaddr::Protocol,
    swarm::{dial_opts::DialOpts, ConnectionId, DialError},
    Multiaddr,
};
use std::{net::ToSocketAddrs, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tracing::{info, warn};

const BACKOFF_DELAY: u64 = 500;
const BACKOFF_MAX_RETRIES: u32 = 10;
const CONNECTION_TIMEOUT: u64 = 60;

#[derive(Message)]
#[rtype(result = "Result<()>")]
pub struct DialPeer(pub String);

#[derive(Clone)]
struct PendingConnection {
    id: ConnectionId,
    addr: String,
    attempt: u32,
    delay_ms: u64,
}

#[derive(Clone)]
pub struct Dialer {
    net_bus: Addr<EventBus<NetworkPeerEvent>>,
    tx: mpsc::Sender<NetworkPeerCommand>,
    pending_connection: Option<PendingConnection>,
    target_addr: String,
}

impl Dialer {
    pub fn new(
        net_bus: Addr<EventBus<NetworkPeerEvent>>,
        tx: mpsc::Sender<NetworkPeerCommand>,
        target_addr: String,
    ) -> Addr<Self> {
        let addr = Self {
            net_bus: net_bus.clone(),
            tx,
            pending_connection: None,
            target_addr,
        }
        .start();

        // Listen on all events
        net_bus.do_send(Subscribe {
            event_type: String::from("*"),
            listener: addr.clone().recipient(),
        });

        addr
    }

    pub fn dial_peer(
        addr: String,
        net_bus: Addr<EventBus<NetworkPeerEvent>>,
        tx: mpsc::Sender<NetworkPeerCommand>,
    ) -> Addr<Self> {
        Self::new(net_bus, tx, addr)
    }

    async fn attempt_dial(
        &mut self,
        addr: String,
        attempt: u32,
        delay_ms: u64,
    ) -> Option<ConnectionId> {
        info!("Attempt {}/{} for {}", attempt, BACKOFF_MAX_RETRIES, addr);
        if attempt > 1 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }

        match addr.parse::<Multiaddr>() {
            Ok(multi) => {
                let resolved_multiaddr = match self.get_resolved_multiaddr(&multi) {
                    Ok(addr) => addr,
                    Err(e) => {
                        warn!("Error resolving multiaddr {}: {}", addr, e);
                        return None;
                    }
                };
                let opts: DialOpts = resolved_multiaddr.into();
                let connection_id = opts.connection_id();

                match self.tx.send(NetworkPeerCommand::Dial(opts)).await {
                    Ok(_) => {
                        info!("Dialing {} with connection {}", addr, connection_id);
                        self.pending_connection = Some(PendingConnection {
                            id: connection_id,
                            addr,
                            attempt,
                            delay_ms,
                        });
                        Some(connection_id)
                    }
                    Err(e) => {
                        warn!("Failed to initiate dial: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("Invalid multiaddr {}: {}", addr, e);
                None
            }
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

    fn handle_connection_error(
        &mut self,
        conn: PendingConnection,
        error: Arc<DialError>,
        ctx: &mut Context<Self>,
    ) {
        warn!("Connection error for {}: {}", conn.addr, error);
        if !matches!(error.as_ref(), DialError::NoAddresses { .. }) {
            if conn.attempt < BACKOFF_MAX_RETRIES {
                let mut dialer = self.clone();
                ctx.spawn(
                    async move {
                        dialer
                            .attempt_dial(conn.addr, conn.attempt + 1, conn.delay_ms * 2)
                            .await;
                    }
                    .into_actor(self),
                );
            } else {
                warn!("Permanent failure for {}: {}", conn.addr, error);
            }
        } else {
            warn!("Permanent failure for {}: {}", conn.addr, error);
        }
    }
}

impl Actor for Dialer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut dialer = self.clone();
        let addr = self.target_addr.clone();
        ctx.spawn(
            async move {
                dialer.attempt_dial(addr, 1, BACKOFF_DELAY).await;
            }
            .into_actor(self),
        );
    }
}

impl Handler<NetworkPeerEvent> for Dialer {
    type Result = ();

    fn handle(&mut self, msg: NetworkPeerEvent, ctx: &mut Context<Self>) {
        match msg {
            NetworkPeerEvent::ConnectionEstablished { connection_id } => {
                if let Some(conn) = self.pending_connection.take() {
                    if conn.id == connection_id {
                        info!("Connection Established for {}", conn.addr);
                    }
                }
            }
            NetworkPeerEvent::DialError {
                connection_id,
                error,
            } => {
                if let Some(conn) = self.pending_connection.take() {
                    if conn.id == connection_id {
                        self.handle_connection_error(conn, error, ctx);
                    }
                }
            }
            NetworkPeerEvent::OutgoingConnectionError {
                connection_id,
                error,
            } => {
                if let Some(conn) = self.pending_connection.take() {
                    if conn.id == connection_id {
                        self.handle_connection_error(conn, error, ctx);
                    }
                }
            }
            _ => {}
        }
    }
}
