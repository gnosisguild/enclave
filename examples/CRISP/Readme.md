# CRISP - Coercion-Resistant Impartial Selection Protocol

Welcome to the CRISP project! This document provides a comprehensive guide to setting up and deploying the application both locally. Follow the steps carefully to ensure that all dependencies, services, and components are properly configured.

## Project Structure

```
CRISP
├── Dockerfile - Dockerfile for a local development environment
├── apps
│   ├── client
│   │   ├── libs/wasm/pkg - WebAssembly library package
│   │   ├── public - Static files
│   │   ├── src - React components and source code
│   │   └── [configuration files and README]
│   ├── risc0
│   │   ├── core - Core logic for the RISC Zero zkVM
│   │   ├── host - Host logic for the RISC Zero zkVM
│   │   ├── methods - Guest programs to run on the RISC Zero zkVM
│   ├── server
│   │   ├── src
│   │   │   ├── cli - CLI for interacting with the CRISP server
│   │   │   └── server - Server for interacting with the enclave contracts and the client
│   └── wasm-crypto
├── contracts - Contracts for the CRISP protocol
├── deploy - Deployment scripts
├── docker-compose.yaml
└── scripts
    ├── local_dev - Scripts for local development
    └── tasks - Scripts for tasks to be run inside the docker container
```

## Docker Development

To start the development environment, run the following command:

```sh
pnpm dev:setup
pnpm dev:start
```

To stop the development environment, run the following command:

```sh
pnpm dev:stop
```

## Prerequisites

Before getting started, make sure you have the following tools installed:

- **Rust**
- **RISC Zero toolchain**
- **Foundry** and **Anvil** (for local testnet)
- **Node.js** (for client-side dependencies)
- **Yarn** (as Node package manager)

## Dependencies

### Install Node

You can install Node following the official [documentation](https://nodejs.org/en) or using a Node Version Manager (e.g., [nvm](https://github.com/nvm-sh/nvm)).

### Install Pnpm

You can install Pnpm following the official [documentation](https://pnpm.io/installation).

### Install Rust and Foundry

You need to install Rust and Foundry first. After installation, restart your terminal.

```sh
# Install Rust
curl https://sh.rustup.rs -sSf | sh

# Install Foundry
curl -L https://foundry.paradigm.xyz | bash
```

### Install RISC Zero Toolchain

Next, install `rzup` for the `cargo-risczero` toolchain.

```sh
# Install rzup
curl -L https://risczero.com/install | bash

# Install RISC Zero toolchain
rzup install cargo-risczero
```

Verify the installation was successful by running:

```sh
cargo risczero --version
```

At this point, you should have all the tools required to develop and deploy an application with [RISC Zero](https://www.risczero.com).

## Setting Up the Web App

To set up the CRISP dApp in your local environment, follow these steps:

1. Navigate to the `client` directory:

   ```sh
   cd CRISP/apps/client
   ```

2. Install dependencies:

   ```sh
   pnpm install
   ```

3. Start the development server:

   ```sh
   pnpm dev
   ```

## Setting Up the CRISP Server

Setting up the CRISP server involves several components, but this guide will walk you through each step.

### Step 1: Start a Local Testnet with Anvil

```sh
anvil
```

Keep Anvil running in the terminal, and open a new terminal for the next steps.

### Step 2: Setting Up the Ciphernodes

1. Clone the [Enclave Repo](https://github.com/gnosisguild/enclave):

   ```sh
   git clone https://github.com/gnosisguild/enclave.git
   ```

2. Navigate to the `evm` directory:

   ```sh
   cd enclave/packages/evm
   ```

3. Install dependencies:

   ```sh
   pnpm install
   ```

4. Delete any previous local deployment (if any):

   ```sh
   rm -rf deployments/localhost/
   ```

5. Deploy the contracts on the local testnet:

   ```sh
   pnpm deploy:mocks --network localhost
   ```

After deployment, you will see the addresses for the following contracts:

- Enclave
- Ciphernode Registry
- Naive Registry Filter
- Mock Input Validator
- Mock E3 Program
- Mock Decryption Verifier
- Mock Compute Provider

Note down the first four addresses as they will be needed to configure `risc0`, `local_testnet` and the `server`.

### Step 3: Deploy the RISC Zero Contracts

1. Navigate to the `CRISP/packages/risc0` directory.

---

**Faster Proving w/ Bonsai**

The following steps are optional. You can config [Bonsai](https://dev.risczero.com/api/generating-proofs/remote-proving) for faster proving.

- Set up environment variables by creating a `.cargo` directory and `config.toml` file:

  ```sh
  mkdir .cargo && cd .cargo && touch config.toml
  ```

- Add the following configuration to `config.toml`:

  > **_Note:_** _This requires having access to a Bonsai API Key. To request an API key [complete the form here](https://bonsai.xyz/apply)._

  ```toml
  [env]
  BONSAI_API_KEY="your_api_key"
  BONSAI_API_URL="your_api_url"
  ```

---

2. In the `risc0/script` directory, update the `config.toml` with the deployed contract addresses. The following configuration is based on default deployment addresses using local Anvil node:

   ```toml
   [profile.custom]
   chainId = 31337
   riscZeroVerifierAddress = "0x0000000000000000000000000000000000000000"
   enclaveAddress = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
   inputValidatorAddress = "0xa513E6E4b8f2a923D98304ec87F64353C4D5C853"
   ```

3. Export the ETH_WALLET_PRIVATE_KEY environment variable (Anvil's default private key):

   ```sh
   export ETH_WALLET_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
   ```

4. Deploy the contracts:

   ```sh
   forge script --rpc-url http://localhost:8545 --broadcast script/Deploy.s.sol
   ```

Note down the `CRISPRisc0` contract Address, which will be used as the E3 Program Address.

### Step 4: Set up Environment Variables

Create a `.env` file in the `server` directory with the following:

```sh
# Private key for the enclave server
PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
ENCLAVE_SERVER_URL=http://0.0.0.0:4000
HTTP_RPC_URL=http://127.0.0.1:8545
WS_RPC_URL=ws://127.0.0.1:8545
CHAIN_ID=31337

# Cron-job API key to trigger new rounds
CRON_API_KEY=1234567890

# Based on Default Anvil Deployments (Only for testing)
ENCLAVE_ADDRESS="0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
CIPHERNODE_REGISTRY_ADDRESS="0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
NAIVE_REGISTRY_FILTER_ADDRESS="0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
E3_PROGRAM_ADDRESS="0x0B306BF915C4d645ff596e518fAf3F9669b97016" # CRISPRisc0 Contract Address

# E3 Config
E3_WINDOW_SIZE=600
E3_THRESHOLD_MIN=1
E3_THRESHOLD_MAX=2
E3_DURATION=600
# E3 Compute Provider Config
E3_COMPUTE_PROVIDER_NAME="RISC0"
E3_COMPUTE_PROVIDER_PARALLEL=false
E3_COMPUTE_PROVIDER_BATCH_SIZE=4 # Must be a power of 2
```

## Running Ciphernodes

Please make sure that your scripts are run from the `enclave` root directory - this is mandatory to set the environment variables correctly. Depending on your operating system, you may need to give additional execution permissions to each script (using `sudo chmod +x script_name.sh`). The following commands assume that `enclave' and `CRISP' share the same parent folder.

Navigate to the root of the `enclave` repository. To run 3 Ciphernodes, use the provided `run_ciphernodes.sh` script.

```sh
./../CRISP/packages/local_testnet/run_ciphernodes.sh
```

After starting the ciphernodes, a new directory `enclave_data` will be created under `CRISP/packages/local_testnet`. This is where all your ciphernodes configs, data and aggregator live. If you need to rebuild your ciphernodes, we suggest you delete this directory (e.g. `rm -rf CRISP/packages/local_testnet/enclave_data`).

Open a new terminal. Navigate to the root of the `enclave` repository and run the aggregator using the `run_aggregator.sh` script:

```sh
./../CRISP/packages/local_testnet/run_aggregator.sh
```

Once the aggregator is running, you can add the Ciphernodes to the registry. Open a new terminal. Navigate to the root of the `enclave` repository and run the following script `add_ciphernodes.sh`:

```sh
./../CRISP/packages/local_testnet/add_ciphernodes.sh
```

## Running the CRISP Server

To run the CRISP Server, open a new terminal and navigate to the `server` directory. Then, execute the following command:

```sh
cargo run --bin server
```

## Interacting with CRISP via CLI

Open a new terminal and navigate to the `server` directory. Then, execute the following command:

```sh
cargo run --bin cli
```

Once the CLI client is running, you can interact with the CRISP voting protocol by following these steps:

1. Select `CRISP: Voting Protocol (ETH)` from the menu.

2. To initiate a new CRISP round, choose the option `Initialize new CRISP round`.

3. To vote in a CRISP round, choose the option `Participate in an E3 round`.

Ensure all services are running correctly and that components are communicating as expected before starting a new CRISP round.

## Contributing

We welcome and encourage community contributions to this repository. Please ensure that you read and understand the [Contributor License Agreement (CLA)](https://github.com/gnosisguild/CLA) before submitting any contributions.

## Security and Liability

This project is provided **WITHOUT ANY WARRANTY**; without even the implied warranty of **MERCHANTABILITY** or **FITNESS FOR A PARTICULAR PURPOSE**.

## License

This repository is licensed under the [LGPL-3.0+ license](LICENSE).
