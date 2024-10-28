use alloy::primitives::Address;
use anyhow::Result;
use figment::{
    providers::{Format, Serialized, Yaml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct ContractAddresses {
    pub enclave: String,
    pub ciphernode_registry: String,
    pub filter_registry: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChainConfig {
    pub enabled: Option<bool>,
    pub name: String,
    pub rpc_url: String, // We may need multiple per chain for redundancy at a later point
    pub contracts: ContractAddresses,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    /// The chains config
    chains: Vec<ChainConfig>,
    /// The name for the keyfile
    key_file: PathBuf,
    /// The base folder for enclave configuration defaults to `~/.config/enclave` on linux
    config_dir: PathBuf,
    /// The name for the database
    db_file: PathBuf,
    /// Config file name
    config_file: PathBuf,
    /// Used for testing if required
    cwd: PathBuf,
    /// The data dir for enclave defaults to `~/.local/share/enclave`
    data_dir: PathBuf,
    /// Ethereum Address for the node
    address: Option<Address>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            chains: vec![],
            key_file: PathBuf::from("key"),   // ~/.config/enclave/key
            db_file: PathBuf::from("db"),     // ~/.config/enclave/db
            config_dir: OsDirs::config_dir(), // ~/.config/enclave
            data_dir: OsDirs::data_dir(),     // ~/.config/enclave
            config_file: PathBuf::from("config.yaml"), // ~/.config/enclave/config.yaml
            cwd: env::current_dir().unwrap_or_default(),
            address: None,
        }
    }
}

impl AppConfig {
    fn ensure_full_path(&self, dir: &PathBuf, file: &PathBuf) -> PathBuf {
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

    fn resolve_base_dir(&self, base_dir: &PathBuf, default_base_dir: &PathBuf) -> PathBuf {
        if base_dir.is_relative() {
            // ConfigDir is relative and the config file is absolute then use the location of the
            // config file. That way all paths are relative to the config file
            if self.config_file.is_absolute() {
                self.config_file
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

    pub fn use_in_mem_store(&self) -> bool {
        false
    }

    pub fn address(&self) -> Option<Address> {
        self.address
    }

    pub fn data_dir(&self) -> PathBuf {
        normalize_path(self.resolve_base_dir(&self.data_dir, &OsDirs::data_dir()))
    }

    pub fn config_dir(&self) -> PathBuf {
        normalize_path(self.resolve_base_dir(&self.config_dir, &OsDirs::config_dir()))
    }

    pub fn chains(&self) -> &Vec<ChainConfig> {
        &self.chains
    }

    pub fn key_file(&self) -> PathBuf {
        self.ensure_full_path(&self.config_dir(), &self.key_file)
    }

    pub fn db_file(&self) -> PathBuf {
        self.ensure_full_path(&self.data_dir(), &self.db_file)
    }

    pub fn config_file(&self) -> PathBuf {
        self.ensure_full_path(&self.config_dir(), &self.config_file)
    }

    pub fn cwd(&self) -> PathBuf {
        self.cwd.to_owned()
    }
}

/// Load the config at the config_file or the default location if not provided
pub fn load_config(config_file: Option<&str>) -> Result<AppConfig> {
    let mut defaults = AppConfig::default();
    if let Some(file) = config_file {
        defaults.config_file = file.into();
    }

    let config = Figment::from(Serialized::defaults(&defaults))
        .merge(Yaml::file(defaults.config_file()))
        .extract()?;

    Ok(config)
}

/// Utility to normalize paths
fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::Normal(name) => {
                components.push(name);
            }
            std::path::Component::RootDir => {
                components.clear();
                components.push(component.as_os_str());
            }
            std::path::Component::Prefix(prefix) => {
                components.push(prefix.as_os_str());
            }
            std::path::Component::CurDir => {}
        }
    }

    let mut result = PathBuf::new();
    for component in components {
        result.push(component);
    }
    result
}

struct OsDirs;
impl OsDirs {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir().unwrap().join("enclave")
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_local_dir().unwrap().join("enclave")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use figment::Jail;

    #[test]
    fn test_ensure_relative_path() {
        Jail::expect_with(|jail| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/testuser".to_string());
            jail.set_env("HOME", &home);

            let config = AppConfig {
                config_file: format!("{}/docs/myconfig.yaml", &home).into(),
                config_dir: "../foo".into(),
                data_dir: "../bar".into(),
                ..AppConfig::default()
            };

            assert_eq!(
                config.key_file(),
                PathBuf::from(format!("{}/foo/key", home))
            );
            assert_eq!(config.db_file(), PathBuf::from(format!("{}/bar/db", home)));

            Ok(())
        });
    }

    #[test]
    fn test_defaults() {
        Jail::expect_with(|jail| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/testuser".to_string());
            jail.set_env("HOME", &home);

            let config = AppConfig::default();

            assert_eq!(
                config.key_file(),
                PathBuf::from(format!("{}/.config/enclave/key", home))
            );

            assert_eq!(
                config.db_file(),
                PathBuf::from(format!("{}/.local/share/enclave/db", home))
            );

            assert_eq!(
                config.config_file(),
                PathBuf::from(format!("{}/.config/enclave/config.yaml", home))
            );

            assert_eq!(
                config.config_dir(),
                PathBuf::from(format!("{}/.config/enclave/", home))
            );

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
                filename,
                r#"
chains:
  - name: "hardhat"
    rpc_url: "ws://localhost:8545"
    contracts:
      enclave: "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
      ciphernode_registry: "0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
      filter_registry: "0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
"#,
            )?;

            let config: AppConfig = load_config(None).map_err(|err| err.to_string())?;
            let chain = config.chains().first().unwrap();

            assert_eq!(chain.name, "hardhat");
            assert_eq!(chain.rpc_url, "ws://localhost:8545");
            assert_eq!(
                chain.contracts.enclave,
                "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
            );

            Ok(())
        });
    }
}
