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

### Generate Template

```
enclave init ./myproj
```

```
cd ./myproj
```

### Install Dependencies

```bash
pnpm install
```

### 2. Start Local Hardhat Node

```bash
# Terminal 1
pnpm node
```

Enclave contracts should be automatically deployed.

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
