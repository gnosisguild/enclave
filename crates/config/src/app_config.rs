// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::chain_config::ChainConfig;
use crate::load_config::find_in_parent;
use crate::load_config::resolve_config_path;
use crate::paths_engine::PathsEngine;
use crate::paths_engine::DEFAULT_CONFIG_NAME;
use crate::yaml::load_yaml_with_env;
use alloy_primitives::Address;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use figment::{
    providers::{Format, Serialized, Yaml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{collections::HashMap, env, path::PathBuf};

/// Either "aggregator" or "ciphernode"
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum NodeRole {
    /// Aggregator role
    Aggregator {
        pubkey_write_path: Option<PathBuf>,
        plaintext_write_path: Option<PathBuf>,
    },
    /// Ciphernode role
    Ciphernode,
}

impl Default for NodeRole {
    fn default() -> Self {
        NodeRole::Ciphernode
    }
}

/// The structure within the app configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct NodeDefinition {
    /// Ethereum Address for the node
    pub address: Option<Address>,
    /// A list of libp2p multiaddrs to dial to as peers when joining the network
    pub peers: Vec<String>,
    /// The port to use for the quic listener
    pub quic_port: u16,
    /// The name for the database
    pub db_file: PathBuf,
    /// The name for the keyfile
    pub key_file: PathBuf,
    /// The data dir for enclave defaults to `~/.local/share/enclave/{name}`
    pub data_dir: PathBuf,
    /// Override the base folder for enclave configuration defaults to `~/.config/enclave/{name}` on linux
    pub config_dir: PathBuf,
    /// The node role eg. "ciphernode" or "aggregator"
    #[serde(default)]
    pub role: NodeRole,
    /// If a net key has not been set autogenerate one on start
    pub autonetkey: bool,
    /// If a password has not been set autogenerate one on start
    pub autopassword: bool,
    /// If a wallet has not been set autogenerate one on start
    pub autowallet: bool,
}

impl Default for NodeDefinition {
    fn default() -> Self {
        Self {
            peers: vec![], // NOTE: We should look at generation via ipns fetch for the latest nodes
            address: None,
            quic_port: 9091,
            key_file: PathBuf::from("key"), // ~/.config/enclave/key
            db_file: PathBuf::from("db"),   // ~/.config/enclave/db
            config_dir: std::path::PathBuf::new(), // ~/.config/enclave
            data_dir: std::path::PathBuf::new(), // ~/.config/enclave
            role: NodeRole::Ciphernode,
            autonetkey: false,
            autopassword: false,
            autowallet: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct BoundlessConfig {
    /// RPC URL for blockchain (e.g., Sepolia)
    pub rpc_url: String,
    /// Private key for submitting requests
    pub private_key: String,
    /// Pinata JWT for uploading programs/inputs
    #[serde(default)]
    pub pinata_jwt: Option<String>,
    /// Pre-uploaded program URL (if program is already on IPFS)
    #[serde(default)]
    pub program_url: Option<String>,
    /// Submit requests onchain (true) or offchain (false)
    #[serde(default = "default_true")]
    pub onchain: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Risc0Config {
    /// Dev mode: 0 = production, 1 = dev mode (fake proofs)
    #[serde(default)]
    pub risc0_dev_mode: u8,
    /// Boundless configuration
    #[serde(default)]
    pub boundless: Option<BoundlessConfig>,
}

impl Default for Risc0Config {
    fn default() -> Self {
        Risc0Config {
            risc0_dev_mode: 1, // Default to dev mode for safety
            boundless: None,
        }
    }
}

/// Configuration for the program runner
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ProgramConfig {
    risc0: Option<Risc0Config>,
    dev: Option<bool>,
}

impl ProgramConfig {
    pub fn risc0(&self) -> Option<&Risc0Config> {
        self.risc0.as_ref()
    }

    pub fn dev(&self) -> bool {
        if let Some(dev) = self.dev {
            return dev;
        }
        false
    }
}

/// The config actually used throughout the app
#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    /// The name of the node
    name: String,
    /// All the node definitions in the unscoped config
    nodes: HashMap<String, NodeDefinition>,
    /// The chains config
    chains: Vec<ChainConfig>,
    /// Non config peers probably from the CLI
    peers: Vec<String>,
    /// Store all paths in the paths engine
    paths: PathsEngine,
    /// Set the Open Telemetry collector grpc endpoint. Eg. 127.0.0.1:4317
    otel: Option<String>,
    /// If a net key has not been set autogenerate one on start
    autonetkey: bool,
    /// If a password has not been set autogenerate one on start
    autopassword: bool,
    /// If a wallet has not been set autogenerate one on start
    autowallet: bool,
    /// Program config
    program: ProgramConfig,
}

impl AppConfig {
    pub fn try_from_unscoped(
        name: &str,
        config: UnscopedAppConfig,
        default_data_dir: &PathBuf,
        default_config_dir: &PathBuf,
        cwd: &PathBuf,
    ) -> Result<Self> {
        let mut config = config;

        if config.nodes.contains_key("_default") {
            bail!("Cannot use the `_default` node profile name as it is a reserved node name. In order to configure the _default profile use the `node` key in your yaml configuration.");
        }

        // Deliberately clobber default
        config.nodes.insert("_default".to_string(), config.node);

        let Some(node) = config.nodes.get(name) else {
            bail!("Could not find node definition for node '{}'. Did you forget to include it in your configuration?", name);
        };

        let node = node.clone();

        let config_dir_override = (node.config_dir != std::path::PathBuf::new())
            .then_some(&node.config_dir)
            .or_else(|| config.config_dir.as_ref());

        let data_dir_override = (node.data_dir != std::path::PathBuf::new())
            .then_some(&node.data_dir)
            .or_else(|| config.data_dir.as_ref());

        let paths = PathsEngine::new(
            name,
            cwd,
            default_data_dir,
            default_config_dir,
            config.found_config_file.as_ref(),
            config_dir_override,
            data_dir_override,
            Some(&node.db_file),
            Some(&node.key_file),
        );

        Ok(AppConfig {
            name: name.to_owned(),
            nodes: config.nodes,
            chains: config.chains,
            peers: vec![],
            paths,
            otel: config.otel,
            autopassword: node.autopassword,
            autowallet: node.autowallet,
            autonetkey: node.autonetkey,
            program: config.program.unwrap_or_default(),
        })
    }

    /// Add the given peers to the peers vector
    pub fn add_peers(&mut self, peers: Vec<String>) {
        self.peers = combine_unique(&self.peers, &peers)
    }

    /// Get the key_file
    pub fn key_file(&self) -> PathBuf {
        self.paths.key_file()
    }

    /// Get the database file
    pub fn db_file(&self) -> PathBuf {
        self.paths.db_file()
    }

    fn node_def(&self) -> &NodeDefinition {
        // NOTE: on creation an invariant we have is that our node name is an extant key in our
        // nodes datastructure so expect here is ok and we dont have to clone the NodeDefinition
        self.nodes.get(&self.name).expect(&format!(
            "Could not find node definition for node '{}'.",
            &self.name
        ))
    }

    /// Use the in-memory store
    pub fn use_in_mem_store(&self) -> bool {
        // Currently hardcoded to true. In the future we can allow this to be set within the
        // configuration for testing
        false
    }

    /// Get the peers list
    pub fn peers(&self) -> Vec<String> {
        let config_peers = self.node_def().peers.clone();
        let cli_peers = self.peers.clone();
        combine_unique(&config_peers, &cli_peers)
    }

    /// get the quic port
    pub fn quic_port(&self) -> u16 {
        self.node_def().quic_port
    }

    /// Get the config file path
    pub fn config_file(&self) -> PathBuf {
        self.paths.config_file()
    }

    /// Get the chains config
    pub fn chains(&self) -> &Vec<ChainConfig> {
        &self.chains
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Get the open telemetry collector url
    pub fn otel(&self) -> Option<String> {
        self.otel.clone()
    }

    /// Get the node's address
    pub fn address(&self) -> Option<Address> {
        self.node_def().address.clone()
    }

    /// Get a collection containing all the node definitions from the configuration
    pub fn nodes(&self) -> &HashMap<String, NodeDefinition> {
        &self.nodes
    }

    /// Get the node's role and enriched relevant provided configuration
    pub fn role(&self) -> NodeRole {
        match self.node_def().role.clone() {
            NodeRole::Aggregator {
                pubkey_write_path,
                plaintext_write_path,
            } => NodeRole::Aggregator {
                // Normalize paths so that these paths are based on the config dir if they are
                // relative
                pubkey_write_path: pubkey_write_path
                    .as_ref()
                    .map(|p| self.paths.relative_to_config(p)),
                plaintext_write_path: plaintext_write_path
                    .as_ref()
                    .map(|p| self.paths.relative_to_config(p)),
            },
            NodeRole::Ciphernode => NodeRole::Ciphernode,
        }
    }

    /// Get the value of autonetkey
    pub fn autonetkey(&self) -> bool {
        self.autonetkey
    }

    /// Get the value of autowallet
    pub fn autowallet(&self) -> bool {
        self.autowallet
    }

    /// Get the value of autopassword
    pub fn autopassword(&self) -> bool {
        self.autopassword
    }

    pub fn program(&self) -> &ProgramConfig {
        &self.program
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct UnscopedAppConfig {
    /// The chains config
    chains: Vec<ChainConfig>,
    /// The base folder for enclave configuration defaults to `~/.config/enclave` on linux
    config_dir: Option<PathBuf>,
    /// The data dir for enclave defaults to `~/.local/share/enclave`
    data_dir: Option<PathBuf>,
    /// The config file as found before initialization this is for testing purposes and you should
    /// not use this in your configurations
    found_config_file: Option<PathBuf>,
    /// The default node that runs during commands like `enclave start` without supplying the
    /// `--name` argument.
    node: NodeDefinition,
    /// The `nodes` key in configuration
    nodes: HashMap<String, NodeDefinition>,
    /// Set the Open Telemetry collector grpc endpoint. Eg. 127.0.0.1:4317
    otel: Option<String>,
    /// Program config
    program: Option<ProgramConfig>,
}

impl Default for UnscopedAppConfig {
    fn default() -> Self {
        Self {
            chains: vec![],
            config_dir: None,
            data_dir: None,
            node: NodeDefinition::default(),
            found_config_file: None,
            otel: None,
            nodes: HashMap::new(),
            program: None,
        }
    }
}

impl UnscopedAppConfig {
    /// Convert to a scoped configuration using local OS based default configuration
    pub fn into_scoped(self, name: &str) -> Result<AppConfig> {
        Ok(AppConfig::try_from_unscoped(
            name,
            self,
            &OsDirs::data_dir(),
            &OsDirs::config_dir(),
            &env::current_dir()?,
        )?)
    }

    /// Convert to a scoped configuration passing in some injected configuration
    pub fn into_scoped_with_defaults(
        self,
        name: &str,
        default_data_dir: &PathBuf,
        default_config_dir: &PathBuf,
        cwd: &PathBuf,
    ) -> Result<AppConfig> {
        Ok(AppConfig::try_from_unscoped(
            name,
            self,
            default_data_dir,
            default_config_dir,
            cwd,
        )?)
    }
}

/// Value struct for passing configuration from the cli to the configuration
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
struct CliOverrides {
    pub otel: Option<String>,
    pub found_config_file: Option<PathBuf>,
}

/// Load the config at the config_file or the default location if not provided
pub fn load_config(
    name: &str,
    found_config_file: Option<String>,
    otel: Option<String>,
) -> Result<AppConfig> {
    let found_config_file = found_config_file.map(PathBuf::from);

    let resolved_config_path = resolve_config_path(
        find_in_parent,            // finding strategy
        env::current_dir()?,       // cwd
        OsDirs::config_dir(),      // default config folder
        DEFAULT_CONFIG_NAME,       // hardcoded now to enclave.config.yaml
        found_config_file.clone(), // config file we have found to exist
    );

    let loaded_yaml =
        load_yaml_with_env(&resolved_config_path).context("Configuration file not found")?;

    let config: UnscopedAppConfig =
        Figment::from(Serialized::defaults(&UnscopedAppConfig::default()))
            .merge(Yaml::string(&loaded_yaml))
            .merge(Serialized::defaults(&CliOverrides {
                otel,
                found_config_file: Some(resolved_config_path),
            }))
            .extract()
            .context("Could not parse configuration")?;

    Ok(config.into_scoped(name).context(format!(
        "Could not apply scope '{}' to configuration.",
        name
    ))?)
}

pub struct OsDirs;
impl OsDirs {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .expect("Enclave may only be run on an OS that can provide a config dir. See https://docs.rs/dirs for more information.")
            .join("enclave")
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_local_dir()
            .expect("Enclave may only be run on an OS that can provide a data dir. See https://docs.rs/dirs for more information.")
            .join("enclave")
    }
}

// TODO: Put this in a universal utils lib
pub fn combine_unique<T: Eq + std::hash::Hash + Clone + Ord>(a: &[T], b: &[T]) -> Vec<T> {
    let mut combined_set: HashSet<_> = a.iter().cloned().collect();
    combined_set.extend(b.iter().cloned());
    let mut result: Vec<_> = combined_set.into_iter().collect();
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::RpcAuth;
    use figment::Jail;

    #[test]
    fn test_deserialization() -> Result<()> {
        let config_str = r#"
data_dir: "/mydata/enclave"
config_dir: "/myconfig/enclave"
chains:
  - name: "hardhat"
    rpc_url: "ws://localhost:8545"
    rpc_auth:
      type: "Basic"
      credentials:
        username: "testUser"
        password: "testPassword"
    contracts:
      enclave: "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
      ciphernode_registry:
        address: "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
        deploy_block: 1764352873645
      bonding_registry: "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"

node:
  config_dir: "/myconfig/override"
  db_file: "./foo"
  quic_port: 1234

program:
  risc0:
    risc0_dev_mode: 0

nodes:
  ag:
    quic_port: 1235
    peers:
      - "one"
      - "two"
    role:
      type: aggregator
      pubkey_write_path: "./output/pubkey.bin"
      plaintext_write_path: "./output/plaintext.txt"

"#;
        {
            // investigate default serialization
            let unscoped: UnscopedAppConfig = serde_yaml::from_str(config_str).unwrap();
            let config = unscoped
                .into_scoped_with_defaults(
                    "_default",
                    &PathBuf::from("/default/data"),
                    &PathBuf::from("/default/config"),
                    &PathBuf::from("/my/cwd"),
                )
                .unwrap();
            assert_eq!(
                config.db_file(),
                PathBuf::from("/mydata/enclave/_default/foo")
            );
            assert_eq!(
                config.key_file(),
                PathBuf::from("/myconfig/override/_default/key")
            );
            assert_eq!(config.quic_port(), 1234);
            assert_eq!(
                config.program().risc0(),
                Some(&Risc0Config {
                    risc0_dev_mode: 0,
                    boundless: None,
                })
            );
            assert!(config.peers().is_empty());
        };
        {
            // investigate ag serialization
            let unscoped: UnscopedAppConfig = serde_yaml::from_str(config_str).unwrap();
            let config = unscoped
                .into_scoped_with_defaults(
                    "ag",
                    &PathBuf::from("/default/data"),
                    &PathBuf::from("/default/config"),
                    &PathBuf::from("/my/cwd"),
                )
                .unwrap();
            let chain = config.chains().first().unwrap();
            assert_eq!(config.quic_port(), 1235);
            assert_eq!(
                chain.contracts.ciphernode_registry.address(),
                "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
            );
            assert_eq!(config.peers(), vec!["one", "two"]);
            assert_eq!(
                config.config_file(),
                PathBuf::from("/default/config/enclave.config.yaml")
            );
            assert_eq!(config.db_file(), PathBuf::from("/mydata/enclave/ag/db"));
            assert_eq!(config.key_file(), PathBuf::from("/myconfig/enclave/ag/key"));

            // Write paths should be relative to config file if they are relative
            assert_eq!(
                config.role(),
                NodeRole::Aggregator {
                    pubkey_write_path: Some(PathBuf::from("/default/config/output/pubkey.bin")),
                    plaintext_write_path: Some(PathBuf::from(
                        "/default/config/output/plaintext.txt"
                    ))
                }
            );
        };
        Ok(())
    }

    #[test]
    fn test_defaults() {
        Jail::expect_with(|jail| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/testuser".to_string());
            jail.set_env("HOME", &home);

            let config = UnscopedAppConfig::default()
                .into_scoped("_default")
                .map_err(|e| e.to_string())?;

            // Use the actual platform directories instead of hardcoded paths.
            let expected_config_dir = OsDirs::config_dir();
            let expected_data_dir = OsDirs::data_dir();

            assert_eq!(
                config.key_file(),
                expected_config_dir.join("_default").join("key")
            );

            assert_eq!(
                config.db_file(),
                expected_data_dir.join("_default").join("db")
            );

            assert_eq!(
                config.config_file(),
                expected_config_dir.join("enclave.config.yaml")
            );

            assert_eq!(config.role(), NodeRole::Ciphernode);

            Ok(())
        });
    }

    #[test]
    fn test_file_not_found() -> Result<()> {
        let Err(err) = load_config("_default", Some("/nope".to_string()), None) else {
            bail!("error expected");
        };
        let Some(e) = err.downcast_ref::<std::io::Error>() else {
            bail!("io error expected");
        };

        assert_eq!(e.kind(), std::io::ErrorKind::NotFound);

        Ok(())
    }

    #[test]
    fn test_config() {
        Jail::expect_with(|jail| {
            let home = format!("{}", jail.directory().to_string_lossy());
            jail.set_env("HOME", &home);
            jail.set_env("XDG_CONFIG_HOME", &format!("{}/.config", home));

            let expected_config_dir = OsDirs::config_dir();
            let filename = expected_config_dir.join("enclave.config.yaml");
            jail.create_dir(&expected_config_dir)?;
            jail.create_file(
                filename.clone(),
                r#"
chains:
  - name: "hardhat"
    rpc_url: "ws://localhost:8545"
    rpc_auth:
      type: "Basic"
      credentials:
        username: "testUser"
        password: "testPassword"
    contracts:
      enclave: "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
      ciphernode_registry:
        address: "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
        deploy_block: 1764352873645
      bonding_registry: "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
"#,
            )?;

            let mut config = load_config("_default", None, None).map_err(|err| err.to_string())?;

            let mut chain = config.chains().first().unwrap();

            assert_eq!(chain.name, "hardhat");
            assert_eq!(chain.rpc_url, "ws://localhost:8545");
            assert_eq!(
                chain.contracts.enclave.address(),
                "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
            );
            assert_eq!(
                chain.contracts.ciphernode_registry.address(),
                "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
            );
            assert_eq!(
                chain.rpc_auth,
                RpcAuth::Basic {
                    username: "testUser".to_string(),
                    password: "testPassword".to_string(),
                }
            );
            assert_eq!(chain.contracts.enclave.deploy_block(), None);
            assert_eq!(
                chain.contracts.ciphernode_registry.deploy_block(),
                Some(1764352873645)
            );

            jail.create_file(
                filename.clone(),
                r#"
chains:
  - name: "hardhat"
    rpc_url: "ws://localhost:8545"
    contracts:
      enclave: "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
      ciphernode_registry:
        address: "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
        deploy_block: 1764352873645
      bonding_registry: "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
"#,
            )?;
            config = load_config("_default", None, None).map_err(|err| err.to_string())?;
            chain = config.chains().first().unwrap();

            assert_eq!(chain.rpc_auth, RpcAuth::None);

            jail.create_file(
                filename,
                r#"
chains:
  - name: "hardhat"
    rpc_url: "ws://localhost:8545"
    rpc_auth:
      type: "Bearer"
      credentials: "testToken"
    contracts:
      enclave: "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
      ciphernode_registry:
        address: "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
        deploy_block: 1764352873645
      bonding_registry: "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
"#,
            )?;

            config = load_config("_default", None, None).map_err(|err| err.to_string())?;
            chain = config.chains().first().unwrap();
            assert_eq!(chain.rpc_auth, RpcAuth::Bearer("testToken".to_string()));

            Ok(())
        });
    }

    #[test]
    fn test_config_env_vars() {
        Jail::expect_with(|jail| {
            let home = format!("{}", jail.directory().to_string_lossy());
            jail.set_env("HOME", &home);
            jail.set_env("XDG_CONFIG_HOME", &format!("{}/.config", home));
            jail.set_env("TEST_RPC_URL_PORT", "8545");
            jail.set_env("TEST_USERNAME", "envUser");
            jail.set_env("TEST_PASSWORD", "envPassword");
            jail.set_env(
                "TEST_CONTRACT_ADDRESS",
                "0x1234567890123456789012345678901234567890",
            );

            let expected_config_dir = OsDirs::config_dir();
            let filename = expected_config_dir.join("enclave.config.yaml");
            jail.create_dir(&expected_config_dir)?;
            jail.create_file(
                filename,
                r#"
chains:
  - name: "hardhat"
    rpc_url: "ws://test-endpoint:${TEST_RPC_URL_PORT}"
    rpc_auth:
      type: "Basic"
      credentials:
        username: "${TEST_USERNAME}"
        password: "${TEST_PASSWORD}"
    contracts:
      enclave: "${TEST_CONTRACT_ADDRESS}"
      ciphernode_registry:
        address: "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
        deploy_block: 1764352873645
      bonding_registry: "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
"#,
            )?;

            let config = load_config("_default", None, None).map_err(|err| err.to_string())?;
            let chain = config.chains().first().unwrap();

            // Test that environment variables are properly substituted
            assert_eq!(chain.rpc_url, "ws://test-endpoint:8545");
            assert_eq!(
                chain.rpc_auth,
                RpcAuth::Basic {
                    username: "envUser".to_string(),
                    password: "envPassword".to_string(),
                }
            );
            assert_eq!(
                chain.contracts.enclave.address(),
                "0x1234567890123456789012345678901234567890"
            );

            Ok(())
        });
    }
}
