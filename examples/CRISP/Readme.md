# CRISP - Coercion-Resistant Impartial Selection Protocol

CRISP (Coercion-Resistant Impartial Selection Protocol) is a secure protocol for digital decision-making, leveraging fully homomorphic encryption (FHE) and distributed threshold cryptography (DTC) to enable verifiable secret ballots. Built with Enclave, CRISP safeguards democratic systems and decision-making applications against coercion, manipulation, and other vulnerabilities. To learn more about CRISP, you can read our [blog post](https://blog.enclave.gg/crisp-private-voting-secret-ballot-fhe-zkp-mpc/) or visit the [documentation](https://docs.enclave.gg/CRISP/introduction).

## Project Structure

CRISP follows a modern structure with clear separation of concerns, consistent with the Enclave root structure.

```bash
CRISP/
|── client/                  # React frontend application
|── server/                  # Rust coordination server
|── program/                 # RISC Zero computation program
|__ packages/                # JavaScript packages.
|__ crates/                  # Rust crates.
├── circuits/                # Noir circuits for ZK proofs
├── scripts/                 # Development and utility scripts
├── enclave.config.yaml      # Ciphernode configuration
```

You can have an extended explanation of the single folders in the dedicated [documentation](https://docs.enclave.gg/CRISP/introduction#project-structure).

## Prerequisites

Before getting started, ensure you have installed:

- [Rust](https://rust-lang.org/tools/install/)
- [Foundry](https://getfoundry.sh)
- [RiscZero](https://dev.risczero.com/api/zkvm/install)
- [NodeJS](https://nodejs.org/en/download)
- [pnpm](https://pnpm.io)
- [Metamask](https://metamask.io)

### Install Node

You can install Node following the official [documentation](https://nodejs.org/en) or using a Node Version Manager (e.g., [nvm](https://github.com/nvm-sh/nvm)).

### Install Pnpm

You can install Pnpm following the official [documentation](https://pnpm.io/installation).

### Install Metamask

You can add Metamask as extension to your browser following the official [documentation](https://metamask.io).

### Install Rust

You need to install Rust. After installation, restart your terminal.

```sh
# Install Rust
curl https://sh.rustup.rs -sSf | sh

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

## Environment

You need to setup your environment variables for `client/` and `server/`. Just copy and paste the `.env.default` as `.env` and overwrite with your values the following variables (you can leave the others initialized with the default values).

### Client

```bash
VITE_E3_PROGRAM_ADDRESS=0x959922bE3CAee4b8Cd9a407cc3ac1C251C2007B1 # Default E3 program address
```

### Server

```bash
ENCLAVE_ADDRESS="0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
CIPHERNODE_REGISTRY_ADDRESS="0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
NAIVE_REGISTRY_FILTER_ADDRESS="0xDc64a140Aa3E981100a9becA4E685f962f0cF6C9"
E3_PROGRAM_ADDRESS="0x959922bE3CAee4b8Cd9a407cc3ac1C251C2007B1" # CRISPProgram Contract Address
```

These address will be displayed after successfully running the `pnpm dev:up` command in a log that will look like the following:

```bash
Deployments:
----------------------------------------------------------------------
Enclave: 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0
Verifier: 0xA51c1fc2f0D1a1b8494Ed1FE312d7C3a78Ed91C0
InputValidator: 0x610178dA211FEF7D417bC0e6FeD39F05609AD788
CRISPInputValidatorFactory: 0x0DCd1Bf9A1b36cE34237eEaFef220932846BCD82
HonkVerifier: 0x9A676e781A523b5d0C0e43731313A708CB607508
CRISPProgram: 0x959922bE3CAee4b8Cd9a407cc3ac1C251C2007B1
```

If you find any inconsistency with the addresses on the environment, you must update them and run the script again (they must match).

## Quick Start

The fastest way to get CRISP running is using the scripts provided in the `scripts/` directory:

```bash
# Setup and build the development environment
pnpm dev:setup

# Start all services (Anvil, Ciphernodes, Applications)
pnpm dev:up
```

This will start all CRISP components:

- Hardhat node (local blockchain)
- Deploy all contracts
- Compile all ZK circuits
- Ciphernodes network
- CRISP applications (server, client)

Additionally, other specific commands are:

```bash
# Rebuild crates, compiles contracts and build the client app
pnpm dev:build

# Invoke the Server CLI
pnpm cli
```

Once everything is running, you can:

1. Navigate `http://localhost:3000` for the client interface
2. Add the Hardhat private key to your wallet: `0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80`
3. Press `Connect Wallet` button and complete the association with your MetaMask account
4. Switch to `Hardhat` local network (this will be handled automatically by the app. You just need to press on the connected account on the frontend and select the network. Then, complete the configuration on MetaMask pop-up).
5. Open a new terminal, run `pnpm cli` and start a new E3 Round.
6. Refresh and interact with the round following the Client interface.

## Manual Start

### Setting Up the Web App

To set up the CRISP dApp in your local environment, follow these steps:

1. Navigate to the `client` directory:

   ```sh
   cd examples/CRISP/client
   ```

2. Start the development server:

   ```sh
   pnpm dev
   ```

### Setting Up the CRISP Server

Setting up the CRISP server involves several components, but this guide will walk you through each step.

#### Step 1: Start a Local Testnet with Anvil

```sh
anvil
```

Keep Anvil running in the terminal, and open a new terminal for the next steps.

#### Step 2: Setting Up the Ciphernodes

1. Clone the [Enclave Repo](https://github.com/gnosisguild/enclave):

   ```sh
   git clone https://github.com/gnosisguild/enclave.git
   ```

2. Navigate to the `examples/CRISP` directory inside the cloned repository:

   ```sh
   cd enclave/examples/CRISP
   ```

3. Deploy the contracts:

   ```sh
   pnpm deploy:contracts:full
   ```

After deployment, you will see the addresses for the following contracts:

- Enclave
- Ciphernode Registry
- Bonding Registry Filter
- Mock Input Validator
- Mock E3 Program
- Mock Decryption Verifier
- Mock Compute Provider
- RISC Zero Verifier
- Honk Verifier
- CRISP Input Validator Factory
- CRISP Program

#### Step 3: RISC0 Setup (Optional)

> Please note that this step is optional for development only. You can run the program server in dev mode which does not use Risc0.
> The smart contracts would have already been deployed at the previous step.

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

#### Step 4: Set up Environment Variables

Create a `.env` file in the `server` directory with the following:

```sh
# Private key for the enclave server
PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
ENCLAVE_SERVER_URL=http://0.0.0.0:4000
HTTP_RPC_URL=http://127.0.0.1:8545
PROGRAM_SERVER_URL=http://127.0.0.1:13151
WS_RPC_URL=ws://127.0.0.1:8545
CHAIN_ID=31337

# Etherscan API key
ETHERSCAN_API_KEY=""

# Cron-job API key to trigger new rounds
CRON_API_KEY=1234567890

# Based on Default Anvil Deployments (Only for testing)
ENCLAVE_ADDRESS="0xA51c1fc2f0D1a1b8494Ed1FE312d7C3a78Ed91C0"
CIPHERNODE_REGISTRY_ADDRESS="0x2279B7A0a67DB372996a5FaB50D91eAA73d2eBe6"
E3_PROGRAM_ADDRESS="0x1613beB3B2C4f22Ee086B2b38C1476A3cE7f78E8" # CRISPProgram Contract Address
FEE_TOKEN_ADDRESS="0x5FbDB2315678afecb367f032d93F642f64180aa3" # Mock ERC20 Token Address

# E3 Config
E3_WINDOW_SIZE=40
E3_THRESHOLD_MIN=1
E3_THRESHOLD_MAX=2
E3_DURATION=160

# E3 Compute Provider Config
E3_COMPUTE_PROVIDER_NAME="RISC0"
E3_COMPUTE_PROVIDER_PARALLEL=false
E3_COMPUTE_PROVIDER_BATCH_SIZE=4 # Must be a power of 2

# ETHERSCAN API Key (optional, leave empty if not using)
ETHERSCAN_API_KEY=""
```

## Running Ciphernodes

To run three ciphernodes, use the following command inside the CRISP directory:

```sh
./scripts/dev_cipher.sh
```

This script will start the ciphernodes, add the ciphernodes to the registry on chain.

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

Ensure all services are running correctly and that components are communicating as expected before starting a new CRISP round.

## Publishing packages to npm

In order to publish a new version of the CRISP packages to npm, you can use:

```sh
pnpm publish:packages x.x.x # where x.x.x is the new version
```

## Contributing

We welcome and encourage community contributions to this repository. Please ensure that you read and understand the [Contributor License Agreement (CLA)](https://github.com/gnosisguild/CLA) before submitting any contributions.

### Branch Cleanup Policy

To help keep the repository clean and maintainable, we automatically delete merged branches after **7 days**.  
You can control this behavior using **PR labels**:

| Label            | Effect                                        |
| ---------------- | --------------------------------------------- |
| `keep-branch`    | ❌ Branch will not be deleted                 |
| `archive-branch` | 🏷️ Branch will be **tagged** and then deleted |
| _no label_       | 🗑️ Branch will be deleted (no tag preserved)  |

> Only apply these labels **before merging** your PR if you want to preserve history or keep the branch alive.

## Security and Liability

This project is provided **WITHOUT ANY WARRANTY**; without even the implied warranty of **MERCHANTABILITY** or **FITNESS FOR A PARTICULAR PURPOSE**.

## License

This repository is licensed under the [LGPL-3.0+ license](LICENSE).
