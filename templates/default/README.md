# Enclave Protocol Template Setup

The Enclave Protocol Template provides a complete development environment for building and testing applications with Fully Homomorphic Encryption (FHE). This template enables local deployment and interaction with the Enclave protocol without requiring the core contracts to be copied.

## Prerequisites

Before getting started, ensure your development environment meets the following requirements:

### Required Software

**Docker** (tested with version 25.0.6 or later)

```bash
docker --version
# Expected output: Docker version 25.0.6, build v25.0.6
```

**Node.js** (version 22.10.0 or later)

```bash
node --version
# Expected output: v22.10.0
```

**pnpm** (version 10.7.1 or later)

```bash
pnpm --version
# Expected output: 10.7.1
```

**Rust** (version 1.85.1 or later)

```bash
rustc --version
# Expected output: rustc 1.85.1 (4eb161250 2025-03-15)
```

### Optional Software

**tmux** (recommended for managing multiple processes)

```bash
tmux -V
# Expected output: tmux 3.4
```

### System Requirements

- Linux/POSIX environment
- For Nix users: A Nix flake is included in the generated template

## Installation

### 1. Install the Enclave CLI

Install the Enclave CLI tool from the official repository:

```bash
cargo install --git https://github.com/gnosisguild/enclave --branch hacknet e3-cli
```

### 2. Install wasm-pack

Install wasm-pack for WebAssembly compilation:

```bash
cargo install wasm-pack
```

## Project Setup

### Generate a New Project

Create a new Enclave project using the CLI:

```bash
enclave init myenclave
cd ./myenclave
```

Replace `myenclave` with your desired project name.

### Project Structure

The generated project contains the following directories and files:

| File/Directory          | Description                                        |
| ----------------------- | -------------------------------------------------- |
| `./client`              | Client-side application                            |
| `./contracts`           | Your contracts that interact with the protocol     |
| `./deploy`              | Your deploy scripts                                |
| `./enclave.config.yaml` | Configuration for the enclave CLI                  |
| `./program`             | FHE computation code                               |
| `./scripts`             | Scripts to run the project                         |
| `./server`              | TypeScript server that coordinates the FHE process |

## Running the Development Environment

### Start All Services

Launch the complete development stack with a single command:

```bash
pnpm dev:all
```

### What Happens Next

The command will start multiple processes simultaneously:

1. **Hardhat EVM Node** - Local Ethereum development network
2. **Enclave Ciphernodes** - Set of nodes for FHE processing
3. **TypeScript Coordination Server** - Manages FHE process coordination
4. **FHE Program Server** - Handles encrypted computation execution
5. **Frontend Application** - User interface for interaction

### Process Management

- **With tmux installed**: Your terminal will split into multiple panes, each showing logs from different services
- **Without tmux**: You'll see a stream of logs from all processes in a single terminal

### Accessing the Application

1. **Wait for initialization**: Allow all processes to fully start and stabilize
2. **Open your browser**: Navigate to [http://localhost:3000](http://localhost:3000)
3. **Configure MetaMask**: Ensure MetaMask is installed and configured with a local network pointing to `http://localhost:8545`

## Next Steps

Once your development environment is running, you can:

- Modify the FHE computation logic in the `./program` directory
- Update smart contracts in the `./contracts` directory
- Customize the client application in the `./client` directory
- Configure deployment scripts in the `./deploy` directory

For detailed usage instructions and API documentation, refer to the project's README.md file and the official Enclave Protocol documentation.
