# Enclave Protocol Template Setup

This template allows you to deploy and interact with the Enclave protocol locally without copying the core contracts.

## Quick Start

### Prerequisites

Tested with the following:

```
docker --version
Docker version 25.0.6, build v25.0.6
```

```
pnpm --version
10.7.1
```

```
node --version
v22.10.0
```

```
rustc --version
rustc 1.85.1 (4eb161250 2025-03-15)
```

Linux/POSIX environment

### Install Enclave

```
cargo install --git https://github.com/gnosisguild/enclave e3-cli
```

### install wasm-pack

```
cargo install wasm-pack
```

### Generate Template

```
enclave init ./myproj
```

```
cd ./myproj
```

### Start Local Hardhat Node

```bash
pnpm node
```

Enclave contracts should be automatically deployed.

### Compiling your program

Use the following command to compile your program:

```
enclave program compile
```

This should create an `ImageID.sol` contract within the `./contracts` folder.

### Your FHE program

Your FHE program is a rust crate located under `./program`.

### Run your program with enclave

To verifiably run your program with FHE locally with enclave you first need to setup an RPC server to receive the computation output.

You RPC server gets called by the enclave program listener when the FHE computation is complete.

We have set one up in the template to run it you can use the following command:

```bash
pnpm rpc
```

Your RPC must provide the following methods:

```ts
type Capabilities = "processOutput" | "shouldCompute";

interface RpcServer {
  // Handle the FHE
  processOutput(e3Id: number, proof: string, ciphertext: string): number;
  capabilities(): Capabilities;
}
```

### Run a listener

Next you can use the `enclave program listen` command to run your computation:

```bash
enclave program listen \
  --json-rpc-server http://localhost:8080 \
  --chain hardhat
```

This will listen to your local hardhat node and trigger computations when the E3 round has expired.

## Usage Commands

### Ciphernode Management

```bash
# Add a ciphernode
pnpm add-ciphernode 0x1234567890123456789012345678901234567890
```

## Alternative: Direct Script Usage

You can also run the scripts directly with custom parameters:

```bash
# Add ciphernode
npx hardhat run scripts/interact.ts -- add-ciphernode 0x1234567890123456789012345678901234567890
```
