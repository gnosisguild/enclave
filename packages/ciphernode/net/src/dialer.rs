use anyhow::Context;
use anyhow::Result;
use futures::future::join_all;
use libp2p::{
    multiaddr::Protocol,
    swarm::{dial_opts::DialOpts, ConnectionId, DialError},
    Multiaddr,
};
use std::net::ToSocketAddrs;
use tokio::sync::{broadcast, mpsc};
use tracing::error;

use crate::{
    events::{NetworkPeerCommand, NetworkPeerEvent},
    retry::{retry_with_backoff, to_retry, RetryError, BACKOFF_DELAY, BACKOFF_MAX_RETRIES},
};

async fn dial_multiaddr(
    cmd_tx: &mpsc::Sender<NetworkPeerCommand>,
    event_tx: &broadcast::Sender<NetworkPeerEvent>,
    multiaddr_str: &str,
) -> Result<()> {
    let multiaddr = &multiaddr_str.parse()?;
    println!("Now dialing in to {}", multiaddr);
    retry_with_backoff(
        || attempt_connection(cmd_tx, event_tx, multiaddr),
        BACKOFF_MAX_RETRIES,
        BACKOFF_DELAY,
    )
    .await?;
    Ok(())
}

fn trace_error(r: Result<()>) {
    if let Err(err) = r {
        error!("{}", err);
    }
}

pub async fn dial_peers(
    cmd_tx: &mpsc::Sender<NetworkPeerCommand>,
    event_tx: &broadcast::Sender<NetworkPeerEvent>,
    peers: &Vec<String>,
) -> Result<()> {
    let futures: Vec<_> = peers
        .iter()
        .map(|addr| dial_multiaddr(cmd_tx, event_tx, addr))
        .collect();
    let results = join_all(futures).await;
    results.into_iter().for_each(trace_error);
    Ok(())
}

async fn attempt_connection(
    cmd_tx: &mpsc::Sender<NetworkPeerCommand>,
    event_tx: &broadcast::Sender<NetworkPeerEvent>,
    multiaddr: &Multiaddr,
) -> Result<(), RetryError> {
    let mut event_rx = event_tx.subscribe();
    let multi = get_resolved_multiaddr(multiaddr).map_err(to_retry)?;
    let opts: DialOpts = multi.clone().into();
    let dial_connection = opts.connection_id();
    println!("Dialing: '{}' with connection '{}'", multi, dial_connection);
    cmd_tx
        .send(NetworkPeerCommand::Dial(opts))
        .await
        .map_err(to_retry)?;
    wait_for_connection(&mut event_rx, dial_connection).await
}

async fn wait_for_connection(
    event_rx: &mut broadcast::Receiver<NetworkPeerEvent>,
    dial_connection: ConnectionId,
) -> Result<(), RetryError> {
    loop {
        match event_rx.recv().await.map_err(to_retry)? {
            NetworkPeerEvent::ConnectionEstablished { connection_id } => {
                if connection_id == dial_connection {
                    println!("Connection Established");
                    return Ok(());
                }
            }
            NetworkPeerEvent::DialError { error } => {
                println!("DialError!");
                return match error.as_ref() {
                    // If we are dialing ourself then we should just fail
                    DialError::NoAddresses { .. } => {
                        println!("DialError received. Returning RetryError::Failure");
                        Err(RetryError::Failure(error.clone().into()))
                    }
                    // Try again otherwise
                    _ => Err(RetryError::Retry(error.clone().into())),
                };
            }
            NetworkPeerEvent::OutgoingConnectionError {
                connection_id,
                error,
            } => {
                println!("OutgoingConnectionError!");
                if connection_id == dial_connection {
                    println!(
                        "Connection {} failed because of error {}. Retrying...",
                        connection_id, error
                    );
                    return match error.as_ref() {
                        // If we are dialing ourself then we should just fail
                        DialError::NoAddresses { .. } => {
                            Err(RetryError::Failure(error.clone().into()))
                        }
                        // Try again otherwise
                        _ => Err(RetryError::Retry(error.clone().into())),
                    };
                }
            }
            _ => (),
        }
    }
}

fn dns_to_ip_addr(original: &Multiaddr, ip_str: &str) -> Result<Multiaddr> {
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

fn extract_dns_host(addr: &Multiaddr) -> Option<String> {
    // Iterate through the protocols in the multiaddr
    for proto in addr.iter() {
        match proto {
            // Match on DNS4 or DNS6 protocols
            Protocol::Dns4(hostname) | Protocol::Dns6(hostname) => {
                return Some(hostname.to_string())
            }
            _ => continue,
        }
    }
    None
}

fn get_resolved_multiaddr(value: &Multiaddr) -> Result<Multiaddr> {
    let maybe_domain = extract_dns_host(value);
    if let Some(domain) = maybe_domain {
        let ip = resolve_ipv4(&domain)?;
        let multi = dns_to_ip_addr(value, &ip)?;
        return Ok(multi);
    } else {
        Ok(value.clone())
    }
}

fn resolve_ipv4(domain: &str) -> Result<String> {
    let addr = format!("{}:0", domain)
        .to_socket_addrs()?
        .find(|addr| addr.ip().is_ipv4())
        .context("no IPv4 addresses found")?;
    Ok(addr.ip().to_string())
}

fn resolve_ipv6(domain: &str) -> Result<String> {
    let addr = format!("{}:0", domain)
        .to_socket_addrs()?
        .find(|addr| addr.ip().is_ipv6())
        .context("no IPv6 addresses found")?;
    Ok(addr.ip().to_string())
}
