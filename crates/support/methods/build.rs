// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Copyright 2023 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{collections::HashMap, env, path::PathBuf};

use risc0_build::{embed_methods_with_options, DockerOptionsBuilder, GuestOptionsBuilder};
use risc0_build_ethereum::generate_solidity_files;

// Paths where the generated Solidity files will be written.
const SOLIDITY_IMAGE_ID_PATH: &str = "../contracts/ImageID.sol";
const SOLIDITY_ELF_PATH: &str = "../tests/Elf.sol";

fn main() {
    // Builds can be made deterministic, and thereby reproducible, by using Docker to build the
    // guest. Check the RISC0_USE_DOCKER variable and use Docker to build the guest if set.
    println!("cargo:rerun-if-env-changed=RISC0_USE_DOCKER");
    println!("cargo:rerun-if-changed=build.rs");
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let mut builder = GuestOptionsBuilder::default();
    if env::var("RISC0_USE_DOCKER").is_ok() {
        let docker_options = DockerOptionsBuilder::default()
            .root_dir(manifest_dir.join("../"))
            .build()
            .unwrap();
        builder.use_docker(docker_options);
    }
    let guest_options = builder.build().unwrap();

    // Generate Rust source files for the methods crate.
    let guests = embed_methods_with_options(HashMap::from([("guests", guest_options)]));

    if std::env::var("SKIP_SOLIDITY").unwrap_or_default() != "1" {
        // Generate Solidity source files for use with Forge.
        let solidity_opts = risc0_build_ethereum::Options::default()
            .with_image_id_sol_path(SOLIDITY_IMAGE_ID_PATH)
            .with_elf_sol_path(SOLIDITY_ELF_PATH);
        generate_solidity_files(guests.as_slice(), &solidity_opts).unwrap();
    } else {
        println!("cargo:warning=Skipping solidity codegen (SKIP_SOLIDITY set)");
    }
}
