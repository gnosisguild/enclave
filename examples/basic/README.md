# Enclave Protocol Template Setup

This template allows you to deploy and interact with the Enclave protocol locally without copying the core contracts.

## Quick Start

### 1. Install Dependencies

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
