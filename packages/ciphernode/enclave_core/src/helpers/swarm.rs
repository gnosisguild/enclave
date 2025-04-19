use anyhow::*;
use std::process::Stdio;
use tokio::process::{Child, Command};

/// Spawn a child process and return the Child handle
pub async fn spawn_process(program: &str, args: Vec<String>) -> Result<Child> {
    let child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    Ok(child)
}
