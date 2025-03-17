# CRISP - Coercion-Resistant Impartial Selection Protocol

Welcome to the CRISP project! This document provides a comprehensive guide to setting up and deploying the application both locally. Follow the steps carefully to ensure that all dependencies, services, and components are properly configured.

## Project Structure

```
CRISP/packages
├── /client/
│   ├── /libs/wasm/pkg/ - WebAssembly library package
│   ├── /public/ - Static files
│   ├── /src/ - React components and source code
│   └── [configuration files and README]
├── /risc0/ - RISC Zero zkVM and Verifier contracts
├── /server/ - Rust server-side logic
└── /web-rust/ - Rust to WebAssembly logic
```

## Prerequisites

Before getting started, make sure you have the following tools installed:

- **Rust**
- **Foundry**
- **RISC Zero toolchain**
- **Node.js** (for client-side dependencies)
- **Anvil** (for local testnet)

## Dependencies

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
rzup
```

Verify the installation was successful by running:

```sh
cargo risczero --version
```

At this point, you should have all the tools required to develop and deploy an application with [RISC Zero](https://www.risczero.com).

## Setting Up the Web App

To set up the CRISP dApp in your local environment, follow these steps:

1. Clone the repository:

   ```sh
   git clone https://github.com/gnosisguild/CRISP.git
   ```

2. Navigate to the `client` directory:

   ```sh
   cd CRISP/packages/client
   ```

3. Install dependencies:

   ```sh
   yarn install
   ```

4. Start the development server:

   ```sh
   yarn dev
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
   yarn install
   ```

4. Deploy the contracts on the local testnet:

   ```sh
   yarn deploy:mocks --network localhost
   ```

After deployment, note down the addresses for the following contracts:

- Enclave
- Ciphernode Registry
- Naive Registry Filter
- Mock Input Validator

### Step 3: Deploy the RISC Zero Contracts

1. Navigate to the `CRISP/packages/risc0` directory.

2. Set up environment variables by creating a `.cargo` directory and `config.toml` file:

   ```sh
   mkdir .cargo && cd .cargo && touch config.toml
   ```

3. Add the following configuration to `config.toml`:

   > **_Note:_** _This requires having access to a Bonsai API Key. To request an API key [complete the form here](https://bonsai.xyz/apply)._

   ```toml
   [env]
   BONSAI_API_KEY="your_api_key"
   BONSAI_API_URL="your_api_url"
   ```

4. In the `risc0/script` directory, update the `config.toml` with the deployed contract addresses:

   ```toml
   [profile.custom]
   chainId = 31337
   riscZeroVerifierAddress = "0x0000000000000000000000000000000000000000"
   enclaveAddress = "your_enclave_address"
   inputValidatorAddress = "your_input_validator_address"
   ```
5. Export the ETH_WALLET_PRIVATE_KEY environment variable:

   ```sh
   export ETH_WALLET_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" # Anvil's default private key
   ```

6. Deploy the contracts:

   ```sh
   forge script --rpc-url http://localhost:8545 --broadcast script/Deploy.s.sol
   ```

Note down the CRISPRisc0 Contract Address, which will be used as the E3 Program Address.

### Step 4: Set up Environment Variables

Create a `.env` file in the `server` directory with the following:

```sh
CRON_API_KEY=your_cron_api_key # Optional for e3_cron binary

PRIVATE_KEY=your_private_key
HTTP_RPC_URL=http://localhost:8545
WS_RPC_URL=ws://localhost:8546
CHAIN_ID=your_chain_id

ENCLAVE_ADDRESS=your_enclave_contract_address
E3_PROGRAM_ADDRESS=your_e3_program_address # CRISPRisc0 Contract Address
CIPHERNODE_REGISTRY_ADDRESS=your_ciphernode_registry_address
NAIVE_REGISTRY_FILTER_ADDRESS=your_naive_registry_filter_address

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

In the root `enclave` directory, you have to run the Ciphernodes. To run 4 Ciphernodes, use the provided script `run_ciphernodes.sh`. Ensure you run the script from the root `enclave` directory to set the environment variables correctly:

```sh
./run_ciphernodes.sh
```

After starting the Ciphernodes, run the aggregator with the script `run_aggregator.sh`:

```sh
./run_aggregator.sh
```

Once the aggregator is running, you can add the Ciphernodes to the registry with the script `add_ciphernodes.sh`:

```sh
./add_ciphernodes.sh
```

## Running the CRISP Server

To run the CRISP Server, navigate to the `server` directory and execute the following command:

```sh
cargo run --bin server
```

## Interacting with CRISP via CLI

Once the CLI client is running, you can interact with the CRISP voting protocol by following these steps:

1. Select `CRISP: Voting Protocol (ETH)` from the menu.

2. To initiate a new CRISP round, choose the option `Initialize new CRISP round`.

Ensure all services are running correctly and that components are communicating as expected before starting a new CRISP round.

## Contributing

We welcome and encourage community contributions to this repository. Please ensure that you read and understand the [Contributor License Agreement (CLA)](https://github.com/gnosisguild/CLA) before submitting any contributions.

## Security and Liability

This project is provided **WITHOUT ANY WARRANTY**; without even the implied warranty of **MERCHANTABILITY** or **FITNESS FOR A PARTICULAR PURPOSE**.

## License

This repository is licensed under the [LGPL-3.0+ license](LICENSE).
