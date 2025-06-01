use anyhow::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process::Stdio, sync::Arc};
use tokio::{
    process::{Child, Command},
    sync::Mutex,
    task::JoinHandle,
};

pub const SERVER_ADDRESS: &str = "127.0.0.1:13415";

/// All the parameters of a command
pub type CommandParams = (String, Vec<String>);
/// A map of all the start commands to manage
pub type CommandMap = HashMap<String, CommandParams>;
/// The management record of the individual process
pub type ProcessRecord = (Child, Vec<JoinHandle<()>>);
/// The map that holds processes
pub type ProcessMap = Arc<Mutex<HashMap<String, ProcessRecord>>>;

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

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum Action {
    Start { id: String },
    Stop { id: String },
    Restart { id: String },
    StartAll,
    StopAll,
    Terminate,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
pub enum Query {
    Success,
    Failure { message: String },
    Status { status: SwarmStatus },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStatus {
    Started,
    Stopped,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwarmStatus {
    pub processes: HashMap<String, ProcessStatus>,
}
