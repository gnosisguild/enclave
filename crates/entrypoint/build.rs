// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Here we build some contract information from the EVM deployment artifacts that we can use within
// our binaries. Specifically we wbuild out a rust file that has a structure we can import and use
// within our configuration builder
use serde_json::{from_reader, Value};
use std::env;
use std::fs::{self, File};
use std::path::Path;

fn main() -> std::io::Result<()> {
    // Get the manifest directory (where Cargo.toml is located)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Path to deployment artifacts
    let deployments_path = Path::new(&manifest_dir)
        .join("..")
        .join("..")
        .join("packages")
        .join("enclave-contracts")
        .join("deployed_contracts.json");

    // Create output string for contract info
    let mut contract_info = String::from(
        "pub struct ContractInfo {\n    pub address: &'static str,\n    pub deploy_block: u64,\n}\n\n"
    );
    contract_info.push_str(
        "pub static CONTRACT_DEPLOYMENTS: phf::Map<&'static str, ContractInfo> = phf::phf_map! {\n",
    );

    // Process each JSON file in the deployments directory
    // for entry in fs::read_dir(deployments_path)? {
    //     let entry = entry?;
    //     let path = entry.path();

    //     if path.extension().and_then(|s| s.to_str()) == Some("json") {
    //         let contract_name = path.file_stem().and_then(|s| s.to_str()).unwrap();

            let file = File::open(&deployments_path)?;
            let json: Value = from_reader(file)?;

            let contract_name = "test";

            info!("Processing json: {}", json);

            // Extract address and block number
            if let (Some(address), Some(deploy_block)) = (
                json["address"].as_str(),
                json["receipt"]["blockNumber"].as_u64(),
            ) {
                contract_info.push_str(&format!(
                    "    \"{}\" => ContractInfo {{\n        address: \"{}\",\n        deploy_block: {},\n    }},\n",
                    contract_name, address, deploy_block
                ));
            }
    //     }
    // }

    contract_info.push_str("};\n");

    // Write the generated code to a file
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("contract_deployments.rs");
    fs::write(dest_path, contract_info)?;
    println!("cargo:rerun-if-changed=../../packages/enclave-contracts/deployed_contracts.json");

    Ok(())
}
