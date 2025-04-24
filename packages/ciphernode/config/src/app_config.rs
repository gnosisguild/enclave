use alloy::primitives::Address;
use anyhow::bail;
use anyhow::Result;
use figment::{
    providers::{Format, Serialized, Yaml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{collections::HashMap, env, path::PathBuf};
use tracing::info;
use tracing::instrument;

use crate::chain_config::ChainConfig;
use crate::load_config::find_in_parent;
use crate::load_config::resolve_config_path;
use crate::paths_engine::PathsEngine;
use crate::paths_engine::DEFAULT_CONFIG_NAME;
use crate::yaml::load_yaml_with_env;

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
    /// Whether to enable mDNS discovery
    pub enable_mdns: bool,
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
}

impl Default for NodeDefinition {
    fn default() -> Self {
        Self {
            peers: vec![], // NOTE: We should look at generation via ipns fetch for the latest nodes
            address: None,
            quic_port: 9091,
            enable_mdns: false,
            key_file: PathBuf::from("key"),   // ~/.config/enclave/key
            db_file: PathBuf::from("db"),     // ~/.config/enclave/db
            config_dir: OsDirs::config_dir(), // ~/.config/enclave
            data_dir: OsDirs::data_dir(),     // ~/.config/enclave
            role: NodeRole::Ciphernode,
        }
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
}

impl AppConfig {
    pub fn try_from_unscoped(
        name: &str,
        config: UnscopedAppConfig,
        default_data_dir: &PathBuf,
        default_config_dir: &PathBuf,
        cwd: &PathBuf,
    ) -> Result<Self> {
        let Some(node) = config.nodes.get(name) else {
            bail!("Could not find node definition for node '{}'. Did you forget to include it in your configuration?", name);
        };
        let paths = PathsEngine::new(
            name,
            cwd,
            default_data_dir,
            default_config_dir,
            config.config_dir.as_ref(),
            config.found_config_file.as_ref(),
            config.data_dir.as_ref(),
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
        })
    }

    pub fn add_peers(&mut self, peers: Vec<String>) {
        self.peers = combine_unique(&self.peers, &peers)
    }

    pub fn key_file(&self) -> PathBuf {
        self.paths.key_file()
    }

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

    pub fn use_in_mem_store(&self) -> bool {
        false
    }

    pub fn peers(&self) -> Vec<String> {
        let config_peers = self.node_def().peers.clone();
        let cli_peers = self.peers.clone();
        combine_unique(&config_peers, &cli_peers)
    }

    pub fn quic_port(&self) -> u16 {
        self.node_def().quic_port
    }

    pub fn enable_mdns(&self) -> bool {
        false
    }

    pub fn config_file(&self) -> PathBuf {
        self.paths.config_file()
    }

    pub fn chains(&self) -> &Vec<ChainConfig> {
        &self.chains
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn otel(&self) -> Option<String> {
        self.otel.clone()
    }

    pub fn address(&self) -> Option<Address> {
        self.node_def().address.clone()
    }

    pub fn nodes(&self) -> &HashMap<String, NodeDefinition> {
        &self.nodes
    }

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
    /// The `nodes` key in configuration
    nodes: HashMap<String, NodeDefinition>,
    /// Set the Open Telemetry collector grpc endpoint. Eg. 127.0.0.1:4317
    otel: Option<String>,
}

impl Default for UnscopedAppConfig {
    fn default() -> Self {
        Self {
            chains: vec![],
            config_dir: None,
            data_dir: None,
            found_config_file: None,
            otel: None,
            nodes: HashMap::from([("default".to_owned(), NodeDefinition::default())]),
        }
    }
}

impl UnscopedAppConfig {
    pub fn into_scoped(self, name: &str) -> Result<AppConfig> {
        Ok(AppConfig::try_from_unscoped(
            name,
            self,
            &OsDirs::data_dir(),
            &OsDirs::config_dir(),
            &env::current_dir()?,
        )?)
    }
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

    let loaded_yaml = load_yaml_with_env(&resolved_config_path)?;

    let config: UnscopedAppConfig =
        Figment::from(Serialized::defaults(&UnscopedAppConfig::default()))
            .merge(Yaml::string(&loaded_yaml))
            .merge(Serialized::defaults(&CliOverrides {
                otel,
                found_config_file: Some(resolved_config_path),
            }))
            .extract()?;

    Ok(config.into_scoped(name)?)
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
      filter_registry: "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
nodes:
  default:
    db_file: "./foo"
    quic_port: 1234

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
                    "default",
                    &PathBuf::from("/default/data"),
                    &PathBuf::from("/default/config"),
                    &PathBuf::from("/my/cwd"),
                )
                .unwrap();
            assert_eq!(
                config.db_file(),
                PathBuf::from("/mydata/enclave/default/foo")
            );
            assert_eq!(
                config.key_file(),
                PathBuf::from("/myconfig/enclave/default/key")
            );
            assert_eq!(config.quic_port(), 1234);
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
                .into_scoped("default")
                .map_err(|e| e.to_string())?;

            assert_eq!(
                config.key_file(),
                PathBuf::from(format!("{}/.config/enclave/default/key", home))
            );

            assert_eq!(
                config.db_file(),
                PathBuf::from(format!("{}/.local/share/enclave/default/db", home))
            );

            assert_eq!(
                config.config_file(),
                PathBuf::from(format!("{}/.config/enclave/enclave.config.yaml", home))
            );

            assert_eq!(config.role(), NodeRole::Ciphernode);

            Ok(())
        });
    }

    #[test]
    fn test_config() {
        Jail::expect_with(|jail| {
            let home = format!("{}", jail.directory().to_string_lossy());
            jail.set_env("HOME", &home);
            jail.set_env("XDG_CONFIG_HOME", &format!("{}/.config", home));
            let filename = format!("{}/.config/enclave/enclave.config.yaml", home);
            let filedir = format!("{}/.config/enclave", home);
            jail.create_dir(filedir)?;
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
      filter_registry: "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
"#,
            )?;

            let mut config = load_config("default", None, None).map_err(|err| err.to_string())?;

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
      filter_registry: "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
"#,
            )?;
            config = load_config("default", None, None).map_err(|err| err.to_string())?;
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
      filter_registry: "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
"#,
            )?;

            config = load_config("default", None, None).map_err(|err| err.to_string())?;
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

            let filename = format!("{}/.config/enclave/enclave.config.yaml", home);
            let filedir = format!("{}/.config/enclave", home);
            jail.create_dir(filedir)?;
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
      filter_registry: "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
"#,
            )?;

            let config = load_config("default", None, None).map_err(|err| err.to_string())?;
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
