use anyhow::Context;
use anyhow::Result;
use futures::future::join_all;
use libp2p::{
    multiaddr::Protocol,
    swarm::{dial_opts::DialOpts, ConnectionId, DialError},
    Multiaddr,
};
use std::net::ToSocketAddrs;
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{sleep, timeout, Duration};
use tracing::error;
use tracing::info;

use crate::{
    events::{NetworkPeerCommand, NetworkPeerEvent},
    retry::{retry_with_backoff, to_retry, RetryError, BACKOFF_DELAY, BACKOFF_MAX_RETRIES},
};

/// Dial a single Multiaddr with retries and return an error should those retries not work
async fn dial_multiaddr(
    cmd_tx: &mpsc::Sender<NetworkPeerCommand>,
    event_tx: &broadcast::Sender<NetworkPeerEvent>,
    multiaddr_str: &str,
) -> Result<()> {
    let multiaddr = &multiaddr_str.parse()?;
    info!("Now dialing in to {}", multiaddr);
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

/// Initiates connections to multiple network peers
///
/// # Arguments
/// * `cmd_tx` - Sender for network peer commands
/// * `event_tx` - Broadcast sender for peer events
/// * `peers` - List of peer addresses to connect to
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

/// Attempt a connection with retrys to a multiaddr return an error if the connection could not be resolved after the retries.
async fn attempt_connection(
    cmd_tx: &mpsc::Sender<NetworkPeerCommand>,
    event_tx: &broadcast::Sender<NetworkPeerEvent>,
    multiaddr: &Multiaddr,
) -> Result<(), RetryError> {
    let mut event_rx = event_tx.subscribe();
    let multi = get_resolved_multiaddr(multiaddr).map_err(to_retry)?;
    let opts: DialOpts = multi.clone().into();
    let dial_connection = opts.connection_id();
    info!("Dialing: '{}' with connection '{}'", multi, dial_connection);
    cmd_tx
        .send(NetworkPeerCommand::Dial(opts))
        .await
        .map_err(to_retry)?;
    wait_for_connection(&mut event_rx, dial_connection).await
}

/// Wait for results of a retry based on a given correlation id and return the correct variant of
/// RetryError depending on the result from the downstream event
async fn wait_for_connection(
    event_rx: &mut broadcast::Receiver<NetworkPeerEvent>,
    dial_connection: ConnectionId,
) -> Result<(), RetryError> {
    loop {
        // Create a timeout future that can be reset
        select! {
            result = event_rx.recv() => {
                match result.map_err(to_retry)? {
                    NetworkPeerEvent::ConnectionEstablished { connection_id } => {
                        if connection_id == dial_connection {
                            info!("Connection Established");
                            return Ok(());
                        }
                    }
                    NetworkPeerEvent::DialError { error } => {
                        info!("DialError!");
                        return match error.as_ref() {
                            // If we are dialing ourself then we should just fail
                            DialError::NoAddresses { .. } => {
                                info!("DialError received. Returning RetryError::Failure");
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
                        info!("OutgoingConnectionError!");
                        if connection_id == dial_connection {
                            info!(
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
            _ = sleep(Duration::from_secs(60)) => {
                info!("Connection attempt timed out after 60 seconds of no events");
                return Err(RetryError::Retry(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Connection attempt timed out",
                ).into()));
            }
        }
    }
}

/// Convert a Multiaddr to use a specific ip address with the ip4 or ip6 protocol
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

/// Detect the DNS host from a multiaddr
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

/// If the Multiaddr uses a DNS domain look it up and return a multiaddr that uses a resolved IP
/// address
fn get_resolved_multiaddr(value: &Multiaddr) -> Result<Multiaddr> {
    if let Some(domain) = extract_dns_host(value) {
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
