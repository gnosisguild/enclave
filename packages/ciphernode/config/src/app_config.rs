use std::path::PathBuf;

use anyhow::Result;
use figment::{
    providers::{Format, Serialized, Yaml},
    Figment,
};
use serde::{Deserialize, Serialize};

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
    pub chains: Vec<ChainConfig>,
    /// The name for the keyfile
    pub keyfile: String,
    /// The base folder for enclave configuration defaults to `~/.config/enclave` on linux
    pub config_dir: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            chains: vec![],
            keyfile: "keyfile".to_string(), // ~/.config/enclave/pw
            config_dir: dirs::config_dir().unwrap().join("enclave"),
        }
    }
}

impl AppConfig {
    pub fn get_keyfile(&self) -> PathBuf {
        self.config_dir.join(&self.keyfile)
    }
}

pub fn load_config(config_path: &str) -> Result<AppConfig> {
    let config: AppConfig = Figment::from(Serialized::defaults(AppConfig::default()))
        .merge(Yaml::file(config_path))
        .extract()?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use figment::Jail;

    #[test]
    fn test_config() {
        Jail::expect_with(|jail| {
            jail.set_env("HOME", "/home/testuser");
            jail.create_file(
                "config.yaml",
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

            let config: AppConfig = load_config("config.yaml").map_err(|err| err.to_string())?;
            assert_eq!(
                config.get_keyfile(),
                PathBuf::from_str("/home/testuser/.config/enclave/keyfile")
                    .map_err(|e| e.to_string())?
            );
            Ok(())
        });
    }
}
