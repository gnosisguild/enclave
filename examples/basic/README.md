# Enclave Protocol Template Setup

This template allows you to deploy and interact with the Enclave protocol locally without copying the core contracts.

## Quick Start

### 1. Install Dependencies

```bash
npm install
# or
yarn install
```

### 2. Start Local Hardhat Node

```bash
# Terminal 1
npm run node
```

### 3. Deploy Protocol Contracts

```bash
# Terminal 2
npm run deploy
```

## Usage Commands

### Deploy Contracts

```bash
# Deploy to local hardhat network
npm run deploy

# Deploy to running localhost node
npm run deploy -- --network localhost
```

### Ciphernode Management

```bash
# Add a ciphernode
npm run add-ciphernode 0x1234567890123456789012345678901234567890

# Remove a ciphernode (requires siblings from tree proof)
npm run remove-ciphernode 0x1234567890123456789012345678901234567890 "123,456,789"

# Get siblings for removal
npm run get-siblings 0x1234567890123456789012345678901234567890 "0xaddr1,0xaddr2,0xaddr3"
```

### Committee Operations

```bash
# Request a new committee
npm run new-committee
```

## Alternative: Direct Script Usage

You can also run the scripts directly with custom parameters:

```bash
# Add ciphernode
npx hardhat run scripts/interact.ts -- add-ciphernode 0x1234567890123456789012345678901234567890

# Remove ciphernode
npx hardhat run scripts/interact.ts -- remove-ciphernode 0x1234567890123456789012345678901234567890 "123,456"

# Get siblings
npx hardhat run scripts/interact.ts -- get-siblings 0x1234567890123456789012345678901234567890 "0xaddr1,0xaddr2"

# New committee
npx hardhat run scripts/interact.ts -- new-committee
```

## Project Structure

```
template/
├── package.json          # Dependencies on your published package
├── hardhat.config.ts     # Points to contracts in node_modules
├── scripts/
│   ├── deploy-local.ts   # Deploys protocol contracts locally
│   └── interact.ts       # Interaction scripts
└── README.md
```

## Important Notes

1. **Contract Sources**: The contracts are loaded from `node_modules/@gnosis-guild/enclave/contracts`
2. **Deployment Logic**: Uses the deployment functions from your published package
3. **Local Only**: This template is designed for local development and testing
4. **Mock Contracts**: Some operations require mock contracts for testing

## Troubleshooting

### "MockE3Program not deployed" Error

If you get this error when creating a committee, you need to deploy mock contracts first. Add this to your main package or create a separate mocks deployment.

### Contract Not Found

Make sure the `@gnosis-guild/enclave` package is properly installed and contains the expected contract files.

### Network Issues

Ensure your local Hardhat node is running on the correct port (8545) and the network configuration matches.
