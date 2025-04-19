use anyhow::*;
use config::{combine_unique, AppConfig, NodeDefinition};
use std::io::Write;
use std::sync::Arc;
use std::{collections::HashMap, env};
use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, ChildStderr, ChildStdout};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{error, info, instrument};

use crate::helpers::swarm::spawn_process;

/// All the parameters of a command
type CommandParams = (String, Vec<String>);
/// A map of all the start commands to manage
type CommandMap = HashMap<String, CommandParams>;
/// The management record of the individual process
type ProcessRecord = (Child, Vec<JoinHandle<()>>);
/// The map that holds processes
type ProcessMap = Arc<Mutex<HashMap<String, ProcessRecord>>>;

/// Metadata used to workout launch charachteristics for swarm mode
#[derive(Clone, Debug)]
pub struct LaunchCommand {
    pub name: String,
    pub ip: String, // maybe this should be an actual socket addr?
    pub quic_port: u16,
    pub peers: Vec<String>,
}

impl LaunchCommand {
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

    pub fn add_peers(&mut self, nodes: &Vec<LaunchCommand>) {
        let peers: Vec<String> = nodes
            .iter()
            .filter(|n| n.name != self.name)
            .map(|n| n.to_multiaddr_str())
            .collect();
        self.peers = combine_unique(&self.peers, &peers);
    }

    pub fn to_params(
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
) -> Result<CommandMap> {
    let mut exclude_list = exclude.clone();

    // Default should not be part of swarm
    exclude_list.push("default".to_string());

    // Filter all the nodes
    let mut filtered: Vec<LaunchCommand> = nodes
        .iter()
        .filter(|(name, _)| !exclude_list.contains(name))
        .map(|(name, value)| LaunchCommand::from_definition(name, ip, value))
        .collect();

    let peers = filtered.clone();
    for item in filtered.iter_mut() {
        item.add_peers(&peers);
    }

    let mut cmds = HashMap::new();
    for item in filtered.iter() {
        let params = item.to_params(verbose, &maybe_config_string)?;
        cmds.insert(item.name.clone(), params);
    }

    Ok(cmds)
}

/// Forward stdout from child process to parent's stdout
fn forward_stdout(id: &str, stdout: ChildStdout) -> JoinHandle<()> {
    let id = id.to_owned();
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            let n = reader.read_until(b'\n', &mut buffer).await.unwrap_or(0);
            if n == 0 {
                break;
            }

            print!("[{}] {}", id, String::from_utf8_lossy(&buffer));
        }
    })
}

/// Forward stderr from child process to parent's stderr
fn forward_stderr(id: &str, stderr: ChildStderr) -> JoinHandle<()> {
    let id = id.to_owned();
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stderr);
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            let n = reader.read_until(b'\n', &mut buffer).await.unwrap_or(0);
            if n == 0 {
                break;
            }

            eprint!("[{}] {}", id, String::from_utf8_lossy(&buffer));
        }
    })
}

/// Run a single command
async fn run_command(id: &str, program: &str, args: Vec<String>) -> Result<ProcessRecord> {
    let mut handles = vec![];
    let mut child = spawn_process(program, args).await?;

    if let Some(stdout) = child.stdout.take() {
        handles.push(forward_stdout(id, stdout));
    }

    if let Some(stderr) = child.stderr.take() {
        handles.push(forward_stderr(id, stderr));
    }

    Ok((child, handles))
}

/// Run commands as child processes and set up output forwarding
async fn run_commands(commands: &CommandMap, processes: &ProcessMap) -> Result<()> {
    let commands = commands.clone();
    for (id, (program, args)) in commands {
        let record = run_command(&id, &program, args).await?;

        // Store the process
        let mut processes_guard = processes.lock().await;
        processes_guard.insert(id, record);
    }
    Ok(())
}

/// Start a process
async fn start(id: &str, commands: &CommandMap, processes: &ProcessMap) -> Result<()> {
    if processes.lock().await.contains_key(id) {
        bail!("Process {} already running!", id);
    }
    let Some(command) = commands.get(id) else {
        bail!("Bad command {}", id);
    };

    let (program, args) = command.clone();
    let record = run_command(id, &program, args).await?;
    let mut processes_guard = processes.lock().await;
    processes_guard.insert(id.to_owned(), record);

    Ok(())
}

/// Start a process
async fn stop(id: &str, processes: &ProcessMap) -> Result<()> {
    let mut processes_lock = processes.lock().await;
    if !processes_lock.contains_key(id) {
        info!("Cannot stop process that isn't running {}", id);
        return Ok(());
    };
    if let Some(mut process_record) = processes_lock.get_mut(id) {
        terminate_process_record(id, &mut process_record).await;
    }
    Ok(())
}

/// Terminate a process
async fn terminate_process_record(id: &str, process_record: &mut ProcessRecord) {
    let (child, handlers) = process_record;
    for handler in handlers.drain(..) {
        // drop all stdout/in handlers
        handler.abort();
    }

    if let Err(e) = child.kill().await {
        error!("Failed to kill process {}: {}", id, e);
    }

    info!("Terminating process: {}...", id);
    let _ = child.wait().await;
    info!("Process {} terminated.", id);
}

/// Terminate all processes
async fn terminate_processes(processes: &ProcessMap) {
    info!("starting to terminate processes...");
    let keys: Vec<String> = {
        processes
            .lock()
            .await
            .keys()
            .map(|k| k.to_string())
            .collect()
    };

    let mut processes_guard = processes.lock().await;
    for id in keys {
        if let Some(mut process_record) = processes_guard.remove(&id) {
            terminate_process_record(&id, &mut process_record).await;
        }
    }
}

/// Terminate all child processes
async fn terminate_processes_and_exit(processes: &ProcessMap) {
    terminate_processes(processes).await;
    info!("SWARM All processes terminated, exiting");
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}

/// Set up signal handlers for graceful shutdown
fn setup_signal_handlers(manager: &ProcessManager) -> JoinHandle<()> {
    let manager = manager.clone();
    tokio::spawn(async move {
        let mut sigterm =
            signal(SignalKind::terminate()).expect("SWARM Failed to set up SIGTERM handler");
        sigterm.recv().await;

        info!("Received SIGTERM, shutting down all processes...");
        manager.terminate().await
    })
}

#[derive(Debug, Clone)]
struct ProcessManager {
    commands: CommandMap,
    processes: ProcessMap,
}

impl ProcessManager {
    pub async fn start_all(&self) -> Result<()> {
        run_commands(&self.commands, &self.processes).await?;
        Ok(())
    }

    pub async fn start(&self, id: &str) -> Result<()> {
        start(id, &self.commands, &self.processes).await?;
        Ok(())
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        stop(id, &self.processes).await?;
        Ok(())
    }

    pub async fn restart(&self, id: &str) -> Result<()> {
        stop(id, &self.processes).await?;
        start(id, &self.commands, &self.processes).await?;
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<()> {
        terminate_processes(&self.processes).await;
        Ok(())
    }

    pub async fn terminate(&self) {
        terminate_processes_and_exit(&self.processes).await;
    }
}

impl From<CommandMap> for ProcessManager {
    fn from(value: CommandMap) -> Self {
        // TODO: should probably implement a singleton pattern here but rn it doesn't matter
        let processes = Arc::new(Mutex::new(HashMap::new()));
        let manager = Self {
            commands: value,
            processes,
        };

        setup_signal_handlers(&manager);

        manager
    }
}

#[instrument(skip_all)]
pub async fn execute(
    config: &AppConfig,
    _detatch: bool, // TBI
    exclude: Vec<String>,
    verbose: u8,
    maybe_config_string: Option<String>,
) -> Result<()> {
    let command_map = extract_commands(
        config.nodes(),
        "127.0.0.1",
        exclude,
        verbose,
        maybe_config_string,
    )?;

    let process_manager = ProcessManager::from(command_map);
    process_manager.start_all().await?;

    tokio::signal::ctrl_c().await?;

    info!("SWARM: Received Ctrl+C shutting down...");
    process_manager.terminate().await;
    Ok(())
}
