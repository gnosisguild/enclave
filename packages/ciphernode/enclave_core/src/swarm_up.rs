use anyhow::*;
use config::{combine_unique, AppConfig, NodeDefinition};
use std::sync::Arc;
use std::{collections::HashMap, env, process::Stdio};
use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, ChildStderr, ChildStdout, Command};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::Mutex;
use tracing::instrument;

/// All the parameters of a command
type CommandParams = (String, Vec<String>);

/// Metadata used to workout launch charachteristics for swarm mode
#[derive(Clone, Debug)]
pub struct SwarmMeta {
    pub name: String,
    pub ip: String, // maybe this should be an actual socket addr?
    pub quic_port: u16,
    pub peers: Vec<String>,
}

impl SwarmMeta {
    pub fn from_definition(name: &str, ip: &str, definition: &NodeDefinition) -> Self {
        Self {
            name: name.to_owned(),
            ip: ip.to_owned(),
            quic_port: definition.quic_port,
            peers: vec![],
        }
    }

    pub fn to_multiaddr_str(&self) -> String {
        format!("/ip4/{}/udp/{}/quic-v1", self.ip, self.quic_port)
    }

    pub fn add_peers(&mut self, nodes: &Vec<SwarmMeta>) {
        let peers: Vec<String> = nodes
            .iter()
            .filter(|n| n.name != self.name)
            .map(|n| n.to_multiaddr_str())
            .collect();
        self.peers = combine_unique(&self.peers, &peers);
    }

    pub fn to_cmd_args(
        &self,
        verbose: u8,
        maybe_config_string: &Option<String>,
    ) -> Result<CommandParams> {
        let enclave_bin = env::current_exe()?.display().to_string();
        let mut args = vec![];
        args.push("start".to_string());

        args.push("--name".to_string());
        args.push(self.name.clone());

        if let Some(config_string) = maybe_config_string {
            args.push("--config".to_string());
            args.push(config_string.to_string());
        }

        if verbose > 0 {
            args.push(format!("-{}", "v".repeat(verbose as usize))); // -vvv
        }

        for peer in self.peers.iter() {
            args.push("--peer".to_string());
            args.push(peer.to_string());
        }

        Ok((enclave_bin, args))
    }
}

fn extract_commands(
    nodes: &HashMap<String, NodeDefinition>,
    ip: &str,
    exclude: Vec<String>,
    verbose: u8,
    maybe_config_string: Option<String>,
) -> Result<HashMap<String, CommandParams>> {
    let mut exclude_list = exclude.clone();

    // Default should not be part of swarm
    exclude_list.push("default".to_string());

    // Filter all the nodes
    let mut filtered: Vec<SwarmMeta> = nodes
        .iter()
        .filter(|(name, _)| !exclude_list.contains(name))
        .map(|(name, value)| SwarmMeta::from_definition(name, ip, value))
        .collect();

    let peers = filtered.clone();
    for item in filtered.iter_mut() {
        item.add_peers(&peers);
    }

    let mut cmds = HashMap::new();
    for item in filtered.iter() {
        let cmd_args: CommandParams = item.to_cmd_args(verbose, &maybe_config_string)?;
        cmds.insert(item.name.clone(), cmd_args);
    }

    Ok(cmds)
}

/// Spawn a child process and return the Child handle
async fn spawn_process(program: String, args: Vec<String>) -> Result<Child> {
    let child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    Ok(child)
}

/// Forward stdout from child process to parent's stdout
async fn forward_stdout(id: String, stdout: ChildStdout) {
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            let n = reader.read_until(b'\n', &mut buffer).await.unwrap_or(0);
            if n == 0 {
                break;
            }

            print!("[{}:stdout] {}", id, String::from_utf8_lossy(&buffer));
        }
    });
}

/// Forward stderr from child process to parent's stderr
async fn forward_stderr(id: String, stderr: ChildStderr) {
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stderr);
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            let n = reader.read_until(b'\n', &mut buffer).await.unwrap_or(0);
            if n == 0 {
                break;
            }

            eprint!("[{}:stderr] {}", id, String::from_utf8_lossy(&buffer));
        }
    });
}

/// Run commands as child processes and set up output forwarding
async fn run_commands(
    commands: HashMap<String, CommandParams>,
) -> Result<Arc<Mutex<HashMap<String, Child>>>> {
    let processes = Arc::new(Mutex::new(HashMap::new()));

    for (id, (program, args)) in commands {
        // Spawn the process
        let mut child = spawn_process(program, args).await?;

        // Set up output forwarding
        if let Some(stdout) = child.stdout.take() {
            forward_stdout(id.clone(), stdout).await;
        }

        if let Some(stderr) = child.stderr.take() {
            forward_stderr(id.clone(), stderr).await;
        }

        // Store the process
        let mut processes_guard = processes.lock().await;
        processes_guard.insert(id, child);
    }

    Ok(processes)
}

/// Terminate all child processes
async fn terminate_processes(processes: Arc<Mutex<HashMap<String, Child>>>) {
    let mut processes_guard = processes.lock().await;

    for (id, child) in processes_guard.iter_mut() {
        println!("Terminating process: {}", id);

        if let Err(e) = child.kill().await {
            eprintln!("Failed to kill process {}: {}", id, e);
        }
    }

    println!("All processes terminated, exiting");
    std::process::exit(0);
}

/// Set up signal handlers for graceful shutdown
async fn setup_signal_handlers(processes: Arc<Mutex<HashMap<String, Child>>>) {
    // Set up SIGTERM handler
    let processes_term = processes.clone();
    tokio::spawn(async move {
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to set up SIGTERM handler");

        sigterm.recv().await;
        println!("Received SIGTERM, shutting down...");
        terminate_processes(processes_term).await;
    });
}

#[instrument(skip_all)]
pub async fn execute(
    config: &AppConfig,
    _detatch: bool, // TBI
    exclude: Vec<String>,
    verbose: u8,
    maybe_config_string: Option<String>,
) -> Result<()> {
    let cmds = extract_commands(
        config.nodes(),
        "127.0.0.1",
        exclude,
        verbose,
        maybe_config_string,
    )?;

    let processes = run_commands(cmds).await?;

    setup_signal_handlers(processes.clone()).await;

    tokio::signal::ctrl_c().await?;
    terminate_processes(processes).await;

    Ok(())
}
