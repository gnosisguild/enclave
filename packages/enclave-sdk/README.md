# Enclave TypeScript SDK

A powerful, type-safe TypeScript SDK for interacting with Enclave smart contracts. This SDK provides
real-time event listening, contract interaction methods, and comprehensive error handling.

## Features

- **Event-driven architecture**: Listen to smart contract events in real-time
- **Type-safe**: Built with TypeScript and uses generated types from contracts
- **Easy contract interactions**: Simple methods for reading from and writing to contracts
- **React integration**: Includes React hooks for easy frontend integration (via `@enclave-e3/react`)
- **Modular architecture**: Tree-shakeable sub-modules for contracts, events, and encryption
- **Encryption helpers**: Standalone FHE encryption functions with optional ZK proof generation
- **Error handling**: Comprehensive error handling with custom error types
- **Gas estimation**: Built-in gas estimation for transactions
- **Event polling**: Support for both WebSocket and polling-based event listening

## Installation

```bash
pnpm add @enclave-e3/sdk
```

## Quick Start

```typescript
import { EnclaveSDK, EnclaveEventType, RegistryEventType } from '@enclave-e3/sdk'
import { createPublicClient, createWalletClient, http, custom } from 'viem'
import { sepolia } from 'viem/chains'

// Initialize clients
const publicClient = createPublicClient({
  chain: sepolia,
  transport: http('YOUR_RPC_URL'),
})

const walletClient = createWalletClient({
  chain: sepolia,
  transport: custom(window.ethereum),
})

// Create SDK instance
const sdk = new EnclaveSDK({
  publicClient,
  walletClient,
  contracts: {
    enclave: '0x...', // Your Enclave contract address
    ciphernodeRegistry: '0x...', // Your CiphernodeRegistry contract address
    feeToken: '0x...', // Your ERC-20 fee token address
  },
  chain: sepolia,
  thresholdBfvParamsPresetName: 'INSECURE_THRESHOLD_512',
})

// Listen to events with the unified event system
sdk.onEnclaveEvent(EnclaveEventType.E3_REQUESTED, (event) => {
  console.log('E3 Requested:', event.data)
})

sdk.onEnclaveEvent(RegistryEventType.COMMITTEE_REQUESTED, (event) => {
  console.log('Committee Requested:', event.data)
})

// Interact with contracts
const hash = await sdk.requestE3({
  threshold: [1, 3],
  inputWindow: [BigInt(0), BigInt(100)],
  e3Program: '0x...',
  e3ProgramParams: '0x...',
  computeProviderParams: '0x...',
  customParams: '0x...',
})
```

### Factory Method

For a simpler setup (especially on the server), use the static `EnclaveSDK.create()` factory:

```typescript
import { EnclaveSDK } from '@enclave-e3/sdk'
import { sepolia } from 'viem/chains'

const sdk = EnclaveSDK.create({
  rpcUrl: 'wss://sepolia.example.com',
  contracts: {
    enclave: '0x...',
    ciphernodeRegistry: '0x...',
    feeToken: '0x...',
  },
  chain: sepolia,
  privateKey: '0x...', // optional — omit for read-only
  thresholdBfvParamsPresetName: 'INSECURE_THRESHOLD_512',
})
```

The factory auto-detects HTTP vs WebSocket transports and creates the appropriate viem clients.

## Usage within a browser

Usage within a typescript project should work out of the box, however in order to use wasm related
functionality of the SDK within the browser vite you must do the following:

- Use `vite`
- Use the `vite-plugin-top-level-await` plugin
- Use the `vite-plugin-wasm` plugin
- Exclude the `@enclave-e3/wasm` package from bundling optimization.

This will enable `vite` to correctly bundle and serve the wasm bundle we use effectively.

```
import { defineConfig } from 'vite'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'

export default defineConfig({
  // other config ...
  optimizeDeps: {
    exclude: ['@enclave-e3/wasm'],
  },
  plugins: [wasm(), topLevelAwait()],
})
```

## Event System

The SDK uses a unified event system with TypeScript enums for type safety:

### Enclave Events

```typescript
enum EnclaveEventType {
  // E3 Lifecycle
  E3_REQUESTED = 'E3Requested',
  CIPHERTEXT_OUTPUT_PUBLISHED = 'CiphertextOutputPublished',
  PLAINTEXT_OUTPUT_PUBLISHED = 'PlaintextOutputPublished',

  // E3 Program Management
  E3_PROGRAM_ENABLED = 'E3ProgramEnabled',
  E3_PROGRAM_DISABLED = 'E3ProgramDisabled',

  // Encryption Scheme Management
  ENCRYPTION_SCHEME_ENABLED = 'EncryptionSchemeEnabled',
  ENCRYPTION_SCHEME_DISABLED = 'EncryptionSchemeDisabled',

  // Configuration
  CIPHERNODE_REGISTRY_SET = 'CiphernodeRegistrySet',
  MAX_DURATION_SET = 'MaxDurationSet',
  ALLOWED_E3_PROGRAMS_PARAMS_SET = 'AllowedE3ProgramsParamsSet',
  OWNERSHIP_TRANSFERRED = 'OwnershipTransferred',
  INITIALIZED = 'Initialized',
}
```

### Registry Events

```typescript
enum RegistryEventType {
  COMMITTEE_REQUESTED = 'CommitteeRequested',
  COMMITTEE_PUBLISHED = 'CommitteePublished',
  COMMITTEE_FINALIZED = 'CommitteeFinalized',
  ENCLAVE_SET = 'EnclaveSet',
  OWNERSHIP_TRANSFERRED = 'OwnershipTransferred',
  INITIALIZED = 'Initialized',
}
```

### Event Data Structure

Each event follows a consistent structure:

```typescript
interface EnclaveEvent<T extends AllEventTypes> {
  type: T
  data: EventData[T] // Typed based on event type
  log: Log // Raw viem log
  timestamp: Date
  blockNumber: bigint
  transactionHash: string
}
```

## React Integration

The SDK includes a React hook via the `@enclave-e3/react` package:

```bash
pnpm add @enclave-e3/react
```

```typescript
import { useEnclaveSDK } from '@enclave-e3/react'

function MyComponent() {
  const {
    sdk,
    isInitialized,
    error,
    requestE3,
    onEnclaveEvent,
    off,
    EnclaveEventType,
    RegistryEventType,
  } = useEnclaveSDK({
    contracts: {
      enclave: '0x...',
      ciphernodeRegistry: '0x...',
      feeToken: '0x...',
    },
    autoConnect: true,
    thresholdBfvParamsPresetName: 'INSECURE_THRESHOLD_512',
  })

  useEffect(() => {
    if (isInitialized) {
      const handler = (event) => {
        console.log('New E3 request:', event)
      }
      onEnclaveEvent(EnclaveEventType.E3_REQUESTED, handler)
      return () => off(EnclaveEventType.E3_REQUESTED, handler)
    }
  }, [isInitialized])

  return (
    <div>
      {error && <p>Error: {error}</p>}
      {!isInitialized && <p>Initializing...</p>}
      {/* Your UI */}
    </div>
  )
}
```

The hook uses wagmi's `usePublicClient` and `useWalletClient` under the hood, so your app must be
wrapped in a wagmi provider.

## Encryption Functions

The SDK provides standalone encryption functions for FHE (Fully Homomorphic Encryption) operations.
These can be used via the SDK instance or imported directly for tree-shaking:

### Via the SDK instance

```typescript
// Generate a public key
const publicKey = await sdk.generatePublicKey()

// Encrypt a single number
const encrypted = await sdk.encryptNumber(42n, publicKey)

// Encrypt a vector
const encryptedVec = await sdk.encryptVector(BigUint64Array.from([1n, 2n, 3n]), publicKey)

// Encrypt with ZK proof generation
const { encryptedData, proof } = await sdk.encryptNumberAndGenProof(42n, publicKey)
```

### Standalone imports

```typescript
import {
  generatePublicKey,
  encryptNumber,
  encryptVector,
  encryptNumberAndGenProof,
  encryptVectorAndGenProof,
  encryptNumberAndGenInputs,
  encryptVectorAndGenInputs,
  computePublicKeyCommitment,
  getThresholdBfvParamsSet,
} from '@enclave-e3/sdk'

const presetName = 'INSECURE_THRESHOLD_512'

const publicKey = await generatePublicKey(presetName)
const encrypted = await encryptNumber(42n, publicKey, presetName)
const { encryptedData, proof } = await encryptNumberAndGenProof(42n, publicKey, presetName)
```

## Modular Imports

The SDK is organized into three sub-modules that can be imported independently for tree-shaking:

```typescript
// Encryption functions and types
import { generatePublicKey, encryptNumber } from '@enclave-e3/sdk/encryption'

// Contract client and types
import { ContractClient } from '@enclave-e3/sdk/contracts'
import type { ContractAddresses, E3 } from '@enclave-e3/sdk/contracts'

// Event listener and types
import { EventListener, EnclaveEventType, RegistryEventType } from '@enclave-e3/sdk/events'
```

All sub-module exports are also re-exported from the main `@enclave-e3/sdk` entry point for convenience.

## API Reference

### Core Methods

#### Contract Interactions

```typescript
// Approve fee token spending
await sdk.approveFeeToken(amount: bigint);

// Request a new E3 computation
await sdk.requestE3({
  threshold: [number, number],
  inputWindow: [bigint, bigint],
  e3Program: `0x${string}`,
  e3ProgramParams: `0x${string}`,
  computeProviderParams: `0x${string}`,
  customParams?: `0x${string}`,
  gasLimit?: bigint
});

// Publish ciphertext output
await sdk.publishCiphertextOutput(e3Id: bigint, ciphertextOutput: `0x${string}`, proof: `0x${string}`, gasLimit?: bigint);

// Read operations
const e3Data = await sdk.getE3(e3Id: bigint);
const publicKey = await sdk.getE3PublicKey(e3Id: bigint);
```

#### Event Handling

```typescript
sdk.onEnclaveEvent(eventType: AllEventTypes, callback: EventCallback);

sdk.off(eventType: AllEventTypes, callback: EventCallback);

sdk.once(eventType: AllEventTypes, callback: EventCallback);

const logs = await sdk.getHistoricalEvents(
  eventType: AllEventTypes,
  fromBlock?: bigint,
  toBlock?: bigint
);

// Event polling (if websockets unavailable)
await sdk.startEventPolling();
sdk.stopEventPolling();
```

#### Encryption

```typescript
// Get BFV parameter set
const params = await sdk.getThresholdBfvParamsSet();

// Key generation
const publicKey = await sdk.generatePublicKey();
const commitment = await sdk.computePublicKeyCommitment(publicKey);

// Encrypt data
const encrypted = await sdk.encryptNumber(data: bigint, publicKey: Uint8Array);
const encryptedVec = await sdk.encryptVector(data: BigUint64Array, publicKey: Uint8Array);

// Encrypt with proof inputs (for ZK verification)
const { encryptedData, circuitInputs } = await sdk.encryptNumberAndGenInputs(data, publicKey);
const { encryptedData, circuitInputs } = await sdk.encryptVectorAndGenInputs(data, publicKey);

// Encrypt with full ZK proof generation
const { encryptedData, proof } = await sdk.encryptNumberAndGenProof(data, publicKey);
const { encryptedData, proof } = await sdk.encryptVectorAndGenProof(data, publicKey);
```

#### Utilities

```typescript
// Gas estimation
const gas = await sdk.estimateGas(functionName, args, contractAddress, abi, value?);

// Transaction waiting
const receipt = await sdk.waitForTransaction(hash);

// Cleanup
sdk.cleanup();
```

## Configuration

```typescript
interface SDKConfig {
  publicClient: PublicClient
  walletClient?: WalletClient
  contracts: {
    enclave: `0x${string}`
    ciphernodeRegistry: `0x${string}`
    feeToken: `0x${string}`
  }
  chain?: Chain
  thresholdBfvParamsPresetName: ThresholdBfvParamsPresetName
}
```

`thresholdBfvParamsPresetName` must be one of: `'INSECURE_THRESHOLD_512'` or `'SECURE_THRESHOLD_8192'`.

## Error Handling

The SDK includes comprehensive error handling:

```typescript
import { SDKError } from '@enclave-e3/sdk'

try {
  await sdk.requestE3(params)
} catch (error) {
  if (error instanceof SDKError) {
    console.error(`SDK Error (${error.code}): ${error.message}`)
  } else {
    console.error('Unexpected error:', error)
  }
}
```

## Development

### Building the SDK

```bash
cd packages/enclave-sdk
pnpm build
```

### Running the Demo

```bash
cd examples/basic/client
pnpm install
pnpm dev
```

The demo showcases all SDK features including real-time event listening and contract interactions.

### Testing

```bash
cd packages/enclave-sdk
pnpm test
```

## Architecture

The SDK is organized into a modular architecture with three domain-specific sub-modules:

- **EnclaveSDK** (`enclave-sdk.ts`): Main orchestrator class that delegates to sub-modules
- **Contracts** (`contracts/`): `ContractClient` for contract read/write operations, type definitions for contract addresses and E3 data
- **Events** (`events/`): `EventListener` for real-time and historical event subscriptions, typed event enums and data interfaces
- **Encryption** (`encryption/`): Standalone FHE encryption functions, BFV parameter management, ZK proof generation
- **Utils** (`utils.ts`): Helper functions, error classes, encoding utilities

Each sub-module has its own `index.ts` entry point and can be imported independently.

## License

This project is licensed under the MIT License.
