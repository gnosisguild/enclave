use anyhow::*;
use reqwest::Client;
use std::env;

use super::swarm::{spawn_process, Action, Query, SERVER_ADDRESS};

pub async fn get_status() -> Result<Query> {
    let client = Client::new();
    let htres = client
        .get(format!("http://{}/status", SERVER_ADDRESS))
        .send()
        .await?;
    let res: Query = htres.json::<Query>().await?;
    Ok(res)
}

pub async fn send_action(action: &Action) -> Result<Query> {
    let client = Client::new();
    let htres = client
        .post(format!("http://{}/command", SERVER_ADDRESS))
        .json(action)
        .send()
        .await?;

    let res: Query = htres.json::<Query>().await?;

    Ok(res)
}

pub async fn terminate() -> Result<()> {
    send_action(&Action::Terminate).await?;
    Ok(())
}

pub async fn is_ready() -> Result<bool> {
    let Query::Status = get_status().await? else {
        return Ok(false);
    };

    Ok(true)
}

pub async fn start_daemon(
    verbose: u8,
    maybe_config_string: &Option<String>,
    exclude: &Vec<String>,
) -> Result<()> {
    let enclave_bin = env::current_exe()?.display().to_string();

    let mut args = vec![];
    args.push("swarm".to_string());
    args.push("daemon".to_string());
    if let Some(config_string) = maybe_config_string {
        args.push("--config".to_string());
        args.push(config_string.to_string());
    }

    if verbose > 0 {
        args.push(format!("-{}", "v".repeat(verbose as usize))); // -vvv
    }

    if exclude.len() > 0 {
        args.push("--exclude".to_string());
        args.push(exclude.join(","));
    }

    // Start and forget
    spawn_process(&enclave_bin, args).await?;

    Ok(())
}
