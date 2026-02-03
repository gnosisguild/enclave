// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::computation::Configs;
use crate::computation::Toml;
use crate::errors::CircuitsErrors;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Artifacts {
    pub toml: Toml,
    pub configs: Configs,
}

pub trait CircuitCodegen: crate::registry::Circuit {
    type Input;
    type Error;

    /// Generate artifacts for a circuit.
    fn codegen(&self, input: Self::Input) -> Result<Artifacts, Self::Error>;
}

pub fn write_toml(toml: &Toml, path: Option<&Path>) -> Result<(), CircuitsErrors> {
    let toml_path = path.unwrap_or_else(|| Path::new("."));
    let toml_path = toml_path.join("Prover.toml");
    Ok(std::fs::write(toml_path, toml)?)
}

pub fn write_configs(configs: &Configs, path: Option<&Path>) -> Result<(), CircuitsErrors> {
    let configs_path = path.unwrap_or_else(|| Path::new("."));
    let configs_path = configs_path.join("configs.nr");
    Ok(std::fs::write(configs_path, configs)?)
}

pub fn write_artifacts(
    toml: &Toml,
    configs: &Configs,
    path: Option<&Path>,
) -> Result<(), CircuitsErrors> {
    write_toml(&toml, path)?;
    write_configs(&configs, path)?;
    Ok(())
}
