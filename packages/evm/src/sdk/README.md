# Enclave TypeScript SDK

A powerful, type-safe TypeScript SDK for interacting with Enclave smart contracts. This SDK provides real-time event listening, contract interaction methods, and comprehensive error handling.

## Features

- **Event-driven architecture**: Listen to smart contract events in real-time
- **Type-safe**: Built with TypeScript and uses generated types from contracts
- **Easy contract interactions**: Simple methods for reading from and writing to contracts
- **React integration**: Includes React hooks for easy frontend integration
- **Error handling**: Comprehensive error handling with custom error types
- **Gas estimation**: Built-in gas estimation for transactions
- **Event polling**: Support for both WebSocket and polling-based event listening

## Installation

```bash
pnpm add @gnosis-guild/enclave
```

## Quick Start

```typescript
import { EnclaveSDK, EnclaveEventType, RegistryEventType } from '@gnosis-guild/enclave/sdk';
import { createPublicClient, createWalletClient, http, custom } from 'viem';

// Initialize clients
const publicClient = createPublicClient({
  transport: http('YOUR_RPC_URL')
});

const walletClient = createWalletClient({
  transport: custom(window.ethereum)
});

// Create SDK instance
const sdk = new EnclaveSDK({
  publicClient,
  walletClient,
  contracts: {
    enclave: '0x...', // Your Enclave contract address
    ciphernodeRegistry: '0x...' // Your CiphernodeRegistry contract address
  },
  chainId: 1 // Optional
});

// Initialize the SDK
await sdk.initialize();

// Listen to events with the unified event system
sdk.onEnclaveEvent(EnclaveEventType.E3_REQUESTED, (event) => {
  console.log('E3 Requested:', event.data);
});

sdk.onEnclaveEvent(RegistryEventType.CIPHERNODE_ADDED, (event) => {
  console.log('Ciphernode Added:', event.data);
});

// Interact with contracts
const hash = await sdk.requestE3({
  filter: '0x...',
  threshold: [1, 3],
  startWindow: [BigInt(0), BigInt(100)],
  duration: BigInt(3600),
  e3Program: '0x...',
  e3ProgramParams: '0x...',
  computeProviderParams: '0x...'
});
```

## Event System

The SDK uses a unified event system with TypeScript enums for type safety:

### Enclave Events

```typescript
enum EnclaveEventType {
  // E3 Lifecycle
  E3_REQUESTED = 'E3Requested',
  E3_ACTIVATED = 'E3Activated',
  INPUT_PUBLISHED = 'InputPublished',
  CIPHERTEXT_OUTPUT_PUBLISHED = 'CiphertextOutputPublished',
  PLAINTEXT_OUTPUT_PUBLISHED = 'PlaintextOutputPublished',
  
  // E3 Program Management
  E3_PROGRAM_ENABLED = 'E3ProgramEnabled',
  E3_PROGRAM_DISABLED = 'E3ProgramDisabled',
  
  // Configuration
  CIPHERNODE_REGISTRY_SET = 'CiphernodeRegistrySet',
  MAX_DURATION_SET = 'MaxDurationSet',
  // ... more events
}
```

### Registry Events

```typescript
enum RegistryEventType {
  CIPHERNODE_ADDED = 'CiphernodeAdded',
  CIPHERNODE_REMOVED = 'CiphernodeRemoved',
  COMMITTEE_REQUESTED = 'CommitteeRequested',
  COMMITTEE_PUBLISHED = 'CommitteePublished',
  ENCLAVE_SET = 'EnclaveSet',
  // ... more events
}
```

### Event Data Structure

Each event follows a consistent structure:

```typescript
interface EnclaveEvent<T extends AllEventTypes> {
  type: T;
  data: EventData[T]; // Typed based on event type
  log: Log; // Raw viem log
  timestamp: Date;
  blockNumber: bigint;
  transactionHash: string;
}
```

## React Integration

The SDK includes a React hook for easy integration:

```typescript
import { useEnclaveSDK } from '@gnosis-guild/enclave/sdk';

function MyComponent() {
  const {
    sdk,
    isInitialized,
    isConnecting,
    error,
    connectWallet,
    requestE3,
    onEnclaveEvent,
    EnclaveEventType
  } = useEnclaveSDK({
    contracts: {
      enclave: '0x...',
      ciphernodeRegistry: '0x...'
    },
    rpcUrl: 'YOUR_RPC_URL',
    autoConnect: true
  });

  useEffect(() => {
    if (isInitialized) {
      onEnclaveEvent(EnclaveEventType.E3_REQUESTED, (event) => {
        console.log('New E3 request:', event);
      });
    }
  }, [isInitialized]);

  return (
    <div>
      {!isInitialized && (
        <button onClick={connectWallet} disabled={isConnecting}>
          {isConnecting ? 'Connecting...' : 'Connect Wallet'}
        </button>
      )}
      {/* Your UI */}
    </div>
  );
}
```

## API Reference

### Core Methods

#### Contract Interactions

```typescript
// Request a new E3 computation
await sdk.requestE3({
  filter: `0x${string}`,
  threshold: [number, number],
  startWindow: [bigint, bigint],
  duration: bigint,
  e3Program: `0x${string}`,
  e3ProgramParams: `0x${string}`,
  computeProviderParams: `0x${string}`,
  value?: bigint,
  gasLimit?: bigint
});

// Activate an E3 computation
await sdk.activateE3(e3Id: bigint, publicKey: `0x${string}`, gasLimit?: bigint);

// Publish input data
await sdk.publishInput(e3Id: bigint, data: `0x${string}`, gasLimit?: bigint);

// Registry operations
await sdk.addCiphernode(node: `0x${string}`, gasLimit?: bigint);
await sdk.removeCiphernode(node: `0x${string}`, siblingNodes: bigint[], gasLimit?: bigint);

// Read operations
const e3Data = await sdk.getE3(e3Id: bigint);
const ciphernodeData = await sdk.getCiphernode(node: `0x${string}`);
```

#### Event Handling

```typescript
// Listen to events (unified API)
sdk.onEnclaveEvent(eventType: AllEventTypes, callback: EventCallback);

// Remove event listeners
sdk.off(eventType: AllEventTypes, callback: EventCallback);

// Get historical events
const logs = await sdk.getHistoricalEvents(
  eventType: AllEventTypes,
  fromBlock?: bigint,
  toBlock?: bigint
);

// Event polling (if websockets unavailable)
await sdk.startEventPolling();
sdk.stopEventPolling();
```

#### Utilities

```typescript
// Gas estimation
const gas = await sdk.estimateGas(functionName, args, contractAddress, abi, value?);

// Transaction waiting
const receipt = await sdk.waitForTransaction(hash);

// Configuration updates
sdk.updateConfig(newConfig: Partial<SDKConfig>);

// Cleanup
sdk.cleanup();
```

## Configuration

```typescript
interface SDKConfig {
  publicClient: PublicClient;
  walletClient?: WalletClient;
  contracts: {
    enclave: `0x${string}`;
    ciphernodeRegistry: `0x${string}`;
  };
  chainId?: number;
}
```

## Error Handling

The SDK includes comprehensive error handling:

```typescript
import { SDKError } from '@gnosis-guild/enclave/sdk';

try {
  await sdk.requestE3(params);
} catch (error) {
  if (error instanceof SDKError) {
    console.error(`SDK Error (${error.code}): ${error.message}`);
  } else {
    console.error('Unexpected error:', error);
  }
}
```

## Development

### Building the SDK

```bash
cd packages/evm
pnpm compile
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
cd packages/evm
pnpm test
```

## Architecture

The SDK consists of several key components:

- **EnclaveSDK**: Main orchestrator class
- **ContractClient**: Handles contract read/write operations
- **EventListener**: Manages real-time event listening
- **Types**: TypeScript definitions with full type safety
- **Utils**: Helper functions and error classes

## License

This project is licensed under the MIT License. 