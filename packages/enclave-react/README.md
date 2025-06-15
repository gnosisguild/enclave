# @gnosis-guild/enclave-react

React hooks and utilities for Enclave SDK.

## Installation

```bash
npm install @gnosis-guild/enclave-react @gnosis-guild/enclave
# or
yarn add @gnosis-guild/enclave-react @gnosis-guild/enclave
# or
pnpm add @gnosis-guild/enclave-react @gnosis-guild/enclave
```

## Usage

### useEnclaveSDK

A React hook for interacting with the Enclave SDK. This hook provides a clean interface for managing SDK state, handling contract interactions, and listening to events.

```tsx
import { useEnclaveSDK } from '@gnosis-guild/enclave-react';

function MyComponent() {
  const {
    sdk,
    isInitialized,
    error,
    requestE3,
    activateE3,
    publishInput,
    onEnclaveEvent,
    off,
    EnclaveEventType,
    RegistryEventType
  } = useEnclaveSDK({
    autoConnect: true,
    contracts: {
      enclave: '0x...',
      ciphernodeRegistry: '0x...'
    },
    chainId: 1
  });

  // Listen to events
  React.useEffect(() => {
    if (!isInitialized) return;

    const handleE3Requested = (event) => {
      console.log('E3 requested:', event.data);
    };

    onEnclaveEvent(EnclaveEventType.E3_REQUESTED, handleE3Requested);

    return () => {
      off(EnclaveEventType.E3_REQUESTED, handleE3Requested);
    };
  }, [isInitialized, onEnclaveEvent, off, EnclaveEventType]);

  // Request computation
  const handleRequest = async () => {
    try {
      const hash = await requestE3({
        filter: '0x...',
        threshold: [2, 3],
        startWindow: [BigInt(Date.now()), BigInt(Date.now() + 300000)],
        duration: BigInt(1800),
        e3Program: '0x...',
        e3ProgramParams: '0x...',
        computeProviderParams: '0x...',
        value: BigInt('1000000000000000') // 0.001 ETH
      });
      console.log('E3 requested with hash:', hash);
    } catch (error) {
      console.error('Failed to request E3:', error);
    }
  };

  if (error) {
    return <div>Error: {error}</div>;
  }

  if (!isInitialized) {
    return <div>Initializing SDK...</div>;
  }

  return (
    <div>
      <button onClick={handleRequest}>
        Request E3 Computation
      </button>
    </div>
  );
}
```

## Features

- **Automatic Wallet Integration**: Seamlessly integrates with wagmi for wallet management
- **Event Handling**: Simple event subscription and cleanup
- **Error Handling**: Comprehensive error states and messages
- **TypeScript Support**: Full type safety with TypeScript
- **Optimized**: Automatic cleanup and efficient re-renders

## Requirements

- React 18+
- wagmi 2.0+
- viem 2.0+

## API

### useEnclaveSDK(config)

#### Parameters

- `config.autoConnect` (boolean, optional): Automatically initialize SDK when wallet is connected
- `config.contracts` (object, optional): Contract addresses for Enclave and CiphernodeRegistry
- `config.chainId` (number, optional): Chain ID for the network

#### Returns

- `sdk`: The raw SDK instance
- `isInitialized`: Boolean indicating if SDK is ready
- `error`: Error message if initialization failed
- `requestE3`: Function to request E3 computation
- `activateE3`: Function to activate E3 environment
- `publishInput`: Function to publish encrypted inputs
- `onEnclaveEvent`: Function to subscribe to events
- `off`: Function to unsubscribe from events
- `EnclaveEventType`: Event type constants
- `RegistryEventType`: Registry event type constants

## License

MIT 