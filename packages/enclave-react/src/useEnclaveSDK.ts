import { useState, useEffect, useCallback, useRef } from "react";
import { useWalletClient, usePublicClient } from "wagmi";
import {
  EnclaveSDK,
  type SDKConfig,
  type AllEventTypes,
  type EventCallback,
  EnclaveEventType,
  RegistryEventType,
  SDKError,
} from "@gnosis-guild/enclave-sdk";

export interface UseEnclaveSDKConfig {
  contracts?: {
    enclave: `0x${string}`;
    ciphernodeRegistry: `0x${string}`;
  };
  chainId?: number;
  autoConnect?: boolean;
}

export interface UseEnclaveSDKReturn {
  sdk: EnclaveSDK | null;
  isInitialized: boolean;
  error: string | null;
  // Contract interaction methods (only the ones commonly used)
  requestE3: typeof EnclaveSDK.prototype.requestE3;
  activateE3: typeof EnclaveSDK.prototype.activateE3;
  publishInput: typeof EnclaveSDK.prototype.publishInput;
  // Event handling
  onEnclaveEvent: <T extends AllEventTypes>(
    eventType: T,
    callback: EventCallback<T>,
  ) => void;
  off: <T extends AllEventTypes>(
    eventType: T,
    callback: EventCallback<T>,
  ) => void;
  // Event types for convenience
  EnclaveEventType: typeof EnclaveEventType;
  RegistryEventType: typeof RegistryEventType;
}

/**
 * React hook for interacting with Enclave SDK
 *
 * @param config Configuration for the SDK initialization
 * @returns Object containing SDK instance and helper methods
 *
 * @example
 * ```tsx
 * import { useEnclaveSDK } from '@gnosis-guild/enclave-react';
 *
 * function MyComponent() {
 *   const {
 *     sdk,
 *     isInitialized,
 *     error,
 *     requestE3,
 *     onEnclaveEvent
 *   } = useEnclaveSDK({
 *     autoConnect: true,
 *     contracts: {
 *       enclave: '0x...',
 *       ciphernodeRegistry: '0x...'
 *     }
 *   });
 *
 *   // Use the SDK...
 * }
 * ```
 */
export const useEnclaveSDK = (
  config: UseEnclaveSDKConfig,
): UseEnclaveSDKReturn => {
  const [sdk, setSdk] = useState<EnclaveSDK | null>(null);
  const [isInitialized, setIsInitialized] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sdkRef = useRef<EnclaveSDK | null>(null);

  const publicClient = usePublicClient();

  const { data: walletClient } = useWalletClient();
  const initializeSDK = useCallback(async () => {
    try {
      setError(null);

      if (!publicClient) {
        throw new Error("Public client not available");
      }

      if (sdk) {
        sdk.cleanup();
      }

      const sdkConfig: SDKConfig = {
        publicClient: publicClient as SDKConfig["publicClient"],
        walletClient: walletClient as SDKConfig["walletClient"],
        contracts: config.contracts || {
          enclave: "0x0000000000000000000000000000000000000000",
          ciphernodeRegistry: "0x0000000000000000000000000000000000000000",
        },
        chainId: config.chainId,
      };

      const newSdk = new EnclaveSDK(sdkConfig);
      await newSdk.initialize();

      setSdk(newSdk);
      sdkRef.current = newSdk;
      setIsInitialized(true);
    } catch (err: unknown) {
      const errorMessage =
        err instanceof SDKError
          ? `SDK Error (${err.code}): ${err.message}`
          : `Failed to initialize SDK: ${err}`;
      setError(errorMessage);
      console.error("SDK initialization failed:", err);
    }
  }, [publicClient, walletClient, config.contracts, config.chainId]);

  // Initialize SDK when wagmi clients are available
  useEffect(() => {
    if (config.autoConnect && publicClient && !isInitialized) {
      initializeSDK();
    }
  }, [config.autoConnect, publicClient, isInitialized, initializeSDK]);

  // Re-initialize when wallet client changes (connect/disconnect)
  useEffect(() => {
    if (isInitialized && publicClient && walletClient) {
      initializeSDK();
    }
  }, [walletClient, initializeSDK]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (sdkRef.current) {
        sdkRef.current.cleanup();
      }
    };
  }, []);

  // Contract interaction methods
  const requestE3 = useCallback(
    (...args: Parameters<typeof EnclaveSDK.prototype.requestE3>) => {
      if (!sdk) throw new Error("SDK not initialized");
      return sdk.requestE3(...args);
    },
    [sdk],
  );

  const activateE3 = useCallback(
    (...args: Parameters<typeof EnclaveSDK.prototype.activateE3>) => {
      if (!sdk) throw new Error("SDK not initialized");
      return sdk.activateE3(...args);
    },
    [sdk],
  );

  const publishInput = useCallback(
    (...args: Parameters<typeof EnclaveSDK.prototype.publishInput>) => {
      if (!sdk) throw new Error("SDK not initialized");
      return sdk.publishInput(...args);
    },
    [sdk],
  );

  // Event handling methods
  const onEnclaveEvent = useCallback(
    <T extends AllEventTypes>(eventType: T, callback: EventCallback<T>) => {
      if (!sdk) throw new Error("SDK not initialized");
      return sdk.onEnclaveEvent(eventType, callback);
    },
    [sdk],
  );

  const off = useCallback(
    <T extends AllEventTypes>(eventType: T, callback: EventCallback<T>) => {
      if (!sdk) throw new Error("SDK not initialized");
      return sdk.off(eventType, callback);
    },
    [sdk],
  );

  return {
    sdk,
    isInitialized,
    error,
    requestE3,
    activateE3,
    publishInput,
    onEnclaveEvent,
    off,
    EnclaveEventType,
    RegistryEventType,
  };
};
