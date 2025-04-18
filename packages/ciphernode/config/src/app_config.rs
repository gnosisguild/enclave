use alloy::primitives::Address;
use anyhow::bail;
use anyhow::Result;
use figment::{
    providers::{Format, Serialized, Yaml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, path::PathBuf};

use crate::chain_config::ChainConfig;
use crate::normalize_path::base_dir;
use crate::normalize_path::normalize_path;
use crate::normalize_path::relative_to;
use crate::yaml::load_yaml_with_env;

fn resolve_base_dir(
    config_file: &PathBuf,
    base_dir: &PathBuf,
    default_base_dir: &PathBuf,
) -> PathBuf {
    if base_dir.is_relative() {
        // ConfigDir is relative and the config file is absolute then use the location of the
        // config file. That way all paths are relative to the config file
        if config_file.is_absolute() {
            config_file
                .parent()
                .map_or_else(|| base_dir.clone(), |p| p.join(base_dir))
        } else {
            // If the config_file is not set but there are relative paths use the default dir use the default dir
            default_base_dir.join(base_dir)
        }
    } else {
        // Use the absolute base_dir
        base_dir.to_owned()
    }
}

fn ensure_full_path(dir: &PathBuf, file: &PathBuf) -> PathBuf {
    normalize_path({
        // If this is absolute return it
        if file.is_absolute() || file.to_string_lossy().starts_with("~") {
            return file.clone();
        }

        // We have to find where it should be relative from
        // Assume it should be the config_dir
        dir.join(file)
    })
}

/// Either "aggregator" or "ciphernode"
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum NodeRole {
    /// Aggregator role
    Aggregator {
        pubkey_write_path: PathBuf,
        plaintext_write_path: PathBuf,
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
    address: Option<Address>,
    /// A list of libp2p multiaddrs to dial to as peers when joining the network
    peers: Vec<String>,
    /// The port to use for the quic listener
    quic_port: u16,
    /// Whether to enable mDNS discovery
    enable_mdns: bool,
    /// The name for the database
    db_file: PathBuf,
    /// The name for the keyfile
    key_file: PathBuf,
    /// The data dir for enclave defaults to `~/.local/share/enclave/{name}`
    data_dir: PathBuf,
    /// Override the base folder for enclave configuration defaults to `~/.config/enclave/{name}` on linux
    config_dir: PathBuf,
    /// The node role eg. "ciphernode" or "aggregator"
    #[serde(default)]
    role: NodeRole,
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
    /// The base folder for enclave configuration defaults to `~/.config/enclave` on linux
    config_dir: PathBuf,
    /// Config file name
    config_file: PathBuf,
    /// Used for testing if required
    cwd: PathBuf,
    /// The data dir for enclave defaults to `~/.local/share/enclave`
    data_dir: PathBuf,
    /// Set the Open Telemetry collector grpc endpoint. Eg. 127.0.0.1:4317
    otel: Option<String>,
}

impl AppConfig {
    pub fn try_from(name: &str, config: UnscopedAppConfig) -> Result<Self> {
        if !config.nodes.contains_key(name) {
            bail!("Could not find node definition for node '{}'. Did you forget to include it in your configuration?", name);
        }

        Ok(AppConfig {
            name: name.to_owned(),
            nodes: config.nodes,
            chains: config.chains,
            config_file: config.config_file,
            config_dir: config.config_dir,
            data_dir: config.data_dir,
            cwd: config.cwd,
            otel: config.otel,
        })
    }

    pub fn key_file(&self) -> PathBuf {
        ensure_full_path(&self.profile_config_dir(), &self.node_def().key_file)
    }

    pub fn db_file(&self) -> PathBuf {
        ensure_full_path(&self.profile_data_dir(), &self.node_def().db_file)
    }

    pub fn data_dir(&self) -> PathBuf {
        normalize_path(resolve_base_dir(
            &self.config_file,
            &self.data_dir,
            &OsDirs::data_dir(),
        ))
    }

    fn node_def(&self) -> &NodeDefinition {
        // NOTE: on creation an invariant we have is that our node name is an extant key in our
        // nodes datastructure so expect here is ok and we dont have to clone the NodeDefinition
        self.nodes.get(&self.name).expect(&format!(
            "Could not find node definition for node '{}'.",
            &self.name
        ))
    }

    fn profile_config_dir(&self) -> PathBuf {
        self.config_dir().join(self.name())
    }

    fn profile_data_dir(&self) -> PathBuf {
        self.data_dir().join(self.name())
    }

    pub fn config_dir(&self) -> PathBuf {
        normalize_path(resolve_base_dir(
            &self.config_file,
            &self.config_dir,
            &OsDirs::config_dir(),
        ))
    }

    pub fn use_in_mem_store(&self) -> bool {
        false
    }

    pub fn peers(&self) -> Vec<String> {
        self.node_def().peers.clone()
    }

    pub fn quic_port(&self) -> u16 {
        self.node_def().quic_port
    }

    pub fn enable_mdns(&self) -> bool {
        false
    }

    pub fn config_file(&self) -> PathBuf {
        ensure_full_path(&self.config_dir(), &self.config_file)
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

    pub fn role(&self) -> NodeRole {
        match self.node_def().role.clone() {
            NodeRole::Aggregator {
                pubkey_write_path,
                plaintext_write_path,
            } => NodeRole::Aggregator {
                pubkey_write_path: normalize_path(relative_to(
                    base_dir(self.config_file()),
                    pubkey_write_path,
                )),
                plaintext_write_path: normalize_path(relative_to(
                    base_dir(self.config_file()),
                    plaintext_write_path,
                )),
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
    config_dir: PathBuf,
    /// Config file name
    config_file: PathBuf,
    /// Used for testing if required
    cwd: PathBuf,
    /// The data dir for enclave defaults to `~/.local/share/enclave`
    data_dir: PathBuf,
    /// The `nodes` key in configuration
    nodes: HashMap<String, NodeDefinition>,
    /// Set the Open Telemetry collector grpc endpoint. Eg. 127.0.0.1:4317
    otel: Option<String>,
}

impl Default for UnscopedAppConfig {
    fn default() -> Self {
        Self {
            chains: vec![],
            config_dir: OsDirs::config_dir(), // ~/.config/enclave
            data_dir: OsDirs::data_dir(),     // ~/.config/enclave
            config_file: PathBuf::from("config.yaml"), // ~/.config/enclave/config.yaml
            cwd: env::current_dir().unwrap_or_default(),
            otel: None,
            nodes: HashMap::from([("default".to_owned(), NodeDefinition::default())]),
        }
    }
}

impl UnscopedAppConfig {
    pub fn into_scoped(self, name: &str) -> Result<AppConfig> {
        Ok(AppConfig::try_from(name, self)?)
    }

    pub fn config_dir(&self) -> PathBuf {
        normalize_path(resolve_base_dir(
            &self.config_file,
            &self.config_dir,
            &OsDirs::config_dir(),
        ))
    }

    pub fn config_file(&self) -> PathBuf {
        ensure_full_path(&self.config_dir(), &self.config_file)
    }
}

/// Override props from the global cli
#[derive(Default, Serialize, Deserialize)]
pub struct CliOverrides {
    pub config: Option<String>,
    pub otel: Option<String>,
}

/// Load the config at the config_file or the default location if not provided
pub fn load_config_from_overrides(name: &str, cli_overrides: CliOverrides) -> Result<AppConfig> {
    let config_file = cli_overrides.config.clone();
    let mut defaults = UnscopedAppConfig::default();
    if let Some(file) = config_file {
        defaults.config_file = file.into();
    }

    let with_envs = load_yaml_with_env(&defaults.config_file())?;

    let config: UnscopedAppConfig = Figment::from(Serialized::defaults(&defaults))
        .merge(Yaml::string(&with_envs))
        .merge(Serialized::defaults(cli_overrides))
        .extract()?;

    Ok(config.into_scoped(name)?)
}

pub fn load_config(name: &str, config_file: Option<String>) -> Result<AppConfig> {
    let mut overrides = CliOverrides::default();
    overrides.config = config_file.clone();
    load_config_from_overrides(name, overrides)
}

struct OsDirs;
impl OsDirs {
    pub fn config_dir() -> PathBuf {
        // TODO: handle unwrap error case
        dirs::config_dir().unwrap().join("enclave")
    }

    pub fn data_dir() -> PathBuf {
        // TODO: handle unwrap error case
        dirs::data_local_dir().unwrap().join("enclave")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::RpcAuth;
    use figment::Jail;

    #[test]
    fn test_normalization() {
        Jail::expect_with(|jail| {
            jail.set_env("HOME", "/home/user");
            let path = normalize_path(&PathBuf::from("~/foo/bar/../baz.txt"));
            assert_eq!(path, PathBuf::from(format!("/home/user/foo/baz.txt")));
            Ok(())
        })
    }

    #[test]
    fn test_ensure_relative_path() {
        Jail::expect_with(|jail| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/testuser".to_string());
            jail.set_env("HOME", &home);

            let config = UnscopedAppConfig {
                config_file: format!("{}/docs/myconfig.yaml", &home).into(),
                config_dir: "../foo".into(),
                data_dir: "../bar".into(),
                ..UnscopedAppConfig::default()
            }
            .into_scoped("default")
            .map_err(|e| e.to_string())?;

            assert_eq!(
                config.key_file(),
                PathBuf::from(format!("{}/foo/default/key", home))
            );
            assert_eq!(
                config.db_file(),
                PathBuf::from(format!("{}/bar/default/db", home))
            );

            Ok(())
        });
    }

    #[test]
    fn test_deserialization() -> Result<()> {
        let config_str = r#"
data_dir: "/home/.local/share/enclave"
config_dir: "/home/.config/enclave"
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
            let config = unscoped.into_scoped("default").unwrap();
            assert_eq!(
                config.db_file(),
                PathBuf::from("/home/.local/share/enclave/default/foo")
            );
            assert_eq!(
                config.key_file(),
                PathBuf::from("/home/.config/enclave/default/key")
            );
            assert_eq!(config.quic_port(), 1234);
            assert!(config.peers().is_empty());
        };
        {
            // investigate ag serialization
            let unscoped: UnscopedAppConfig = serde_yaml::from_str(config_str).unwrap();
            let config = unscoped.into_scoped("ag").unwrap();
            let chain = config.chains().first().unwrap();
            assert_eq!(config.quic_port(), 1235);
            assert_eq!(
                chain.contracts.ciphernode_registry.address(),
                "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
            );
            assert_eq!(config.peers(), vec!["one", "two"]);
            assert_eq!(
                config.config_file(),
                PathBuf::from("/home/.config/enclave/config.yaml")
            );
            assert_eq!(
                config.db_file(),
                PathBuf::from("/home/.local/share/enclave/ag/db")
            );
            assert_eq!(
                config.key_file(),
                PathBuf::from("/home/.config/enclave/ag/key")
            );

            // Write paths should be relative to config file if they are relative
            assert_eq!(
                config.role(),
                NodeRole::Aggregator {
                    pubkey_write_path: PathBuf::from("/home/.config/enclave/output/pubkey.bin"),
                    plaintext_write_path: PathBuf::from(
                        "/home/.config/enclave/output/plaintext.txt"
                    )
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
                PathBuf::from(format!("{}/.config/enclave/config.yaml", home))
            );

            assert_eq!(
                config.config_dir(),
                PathBuf::from(format!("{}/.config/enclave/", home))
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
            let filename = format!("{}/.config/enclave/config.yaml", home);
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

            let mut config = load_config("default", None).map_err(|err| err.to_string())?;

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
            config = load_config("default", None).map_err(|err| err.to_string())?;
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

            config = load_config("default", None).map_err(|err| err.to_string())?;
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

            let filename = format!("{}/.config/enclave/config.yaml", home);
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

            let config = load_config("default", None).map_err(|err| err.to_string())?;
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
