use alloy::primitives::Address;
use anyhow::{Context, Result};
use e3_config::AppConfig;
use serde_yaml::Value;
use std::fs;

type YamlMap = serde_yaml::Mapping;

/// Write an ethereum address into the appropriate node section of the config YAML
pub fn execute(config: &AppConfig, address: Address) -> Result<()> {
    let path = config.config_yaml();
    let updated = apply_address(&config.name(), &fs::read_to_string(&path)?, address)?;
    fs::write(&path, updated)?;
    Ok(())
}

fn apply_address(name: &str, content: &str, address: Address) -> Result<String> {
    let addr = format!("{:?}", address);
    let mut root: Value = serde_yaml::from_str(content)?;

    // Route to either `node.address` (default) or `nodes.<name>.address` (named)
    let map = root
        .as_mapping_mut()
        .context("Expected YAML mapping at root")?;
    if name == "_default" {
        get_or_insert_map(map, "node")
            .insert(Value::String("address".into()), Value::String(addr.clone()));
    } else {
        let nodes = get_or_insert_map(map, "nodes");
        get_or_insert_map(nodes, name)
            .insert(Value::String("address".into()), Value::String(addr.clone()));
    }

    // serde_yaml omits quotes; patch output so addresses are quoted strings
    let yaml = serde_yaml::to_string(&root)?;
    Ok(yaml.replace(&format!("address: {addr}"), &format!("address: \"{addr}\"")))
}

// Retrieve or create a nested mapping under `key`
fn get_or_insert_map<'a>(map: &'a mut YamlMap, key: &str) -> &'a mut YamlMap {
    map.entry(Value::String(key.into()))
        .or_insert_with(|| Value::Mapping(YamlMap::new()))
        .as_mapping_mut()
        .expect("Expected mapping")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const ADDR1: &str = "0x1234567890123456789012345678901234567890";
    const ADDR2: &str = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0";

    #[test]
    fn test_apply_address_default() {
        let result = apply_address(
            "_default",
            r#"node:
  role: ciphernode
"#,
            Address::from_str(ADDR1).unwrap(),
        )
        .unwrap();
        assert_eq!(
            result,
            r#"node:
  role: ciphernode
  address: "0x1234567890123456789012345678901234567890"
"#
        );
    }

    #[test]
    fn test_apply_address_named_node() {
        let result = apply_address(
            "foo",
            r#"node:
  role: ciphernode
nodes:
  foo:
    role: aggregator
"#,
            Address::from_str(ADDR2).unwrap(),
        )
        .unwrap();
        assert_eq!(
            result,
            r#"node:
  role: ciphernode
nodes:
  foo:
    role: aggregator
    address: "0x9fe46736679d2d9a65f0992f2272de9f3c7fa6e0"
"#
        );
    }

    #[test]
    fn test_apply_address_default_existing_address() {
        let result = apply_address(
            "_default",
            r#"node:
  address: "0x0000000000000000000000000000000000000001"
  role: ciphernode
"#,
            Address::from_str(ADDR1).unwrap(),
        )
        .unwrap();
        assert_eq!(
            result,
            r#"node:
  address: "0x1234567890123456789012345678901234567890"
  role: ciphernode
"#
        );
    }

    #[test]
    fn test_apply_address_named_node_existing_address() {
        let result = apply_address(
            "foo",
            r#"nodes:
  foo:
    address: "0x0000000000000000000000000000000000000001"
    role: aggregator
"#,
            Address::from_str(ADDR2).unwrap(),
        )
        .unwrap();
        assert_eq!(
            result,
            r#"nodes:
  foo:
    address: "0x9fe46736679d2d9a65f0992f2272de9f3c7fa6e0"
    role: aggregator
"#
        );
    }

    #[test]
    fn test_apply_address_preserves_other_fields() {
        let result = apply_address(
            "_default",
            r#"data_dir: "/mydata/enclave"
config_dir: "/myconfig/enclave"
chains:
  - name: "hardhat"
    rpc_url: "ws://localhost:8545"
    contracts:
      enclave: "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
node:
  role: ciphernode
  db_file: "./foo"
program:
  risc0:
    risc0_dev_mode: 0
"#,
            Address::from_str(ADDR1).unwrap(),
        )
        .unwrap();
        for s in [
            "data_dir:",
            "config_dir:",
            "chains:",
            "hardhat",
            "program:",
            "risc0_dev_mode: 0",
            "db_file:",
            "address:",
        ] {
            assert!(result.contains(s));
        }
    }
}
