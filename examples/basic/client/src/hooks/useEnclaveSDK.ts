import { useState, useEffect, useCallback, useRef } from 'react';
import { createPublicClient, createWalletClient, custom, http } from 'viem';
import {
    EnclaveSDK,
    type SDKConfig,
    type AllEventTypes,
    type EventCallback,
    EnclaveEventType,
    RegistryEventType,
    SDKError
} from '../sdk';

interface UseEnclaveSDKConfig {
    contracts: {
        enclave: `0x${string}`;
        ciphernodeRegistry: `0x${string}`;
    };
    chainId?: number;
    autoConnect?: boolean;
    rpcUrl: string;
}

interface UseEnclaveSDKReturn {
    sdk: EnclaveSDK | null;
    isInitialized: boolean;
    isConnecting: boolean;
    error: string | null;
    connectWallet: () => Promise<void>;
    // Contract interaction methods
    requestE3: typeof EnclaveSDK.prototype.requestE3;
    activateE3: typeof EnclaveSDK.prototype.activateE3;
    publishInput: typeof EnclaveSDK.prototype.publishInput;
    addCiphernode: typeof EnclaveSDK.prototype.addCiphernode;
    removeCiphernode: typeof EnclaveSDK.prototype.removeCiphernode;
    getE3: typeof EnclaveSDK.prototype.getE3;
    getCiphernode: typeof EnclaveSDK.prototype.getCiphernode;
    // Event handling
    onEnclaveEvent: <T extends AllEventTypes>(eventType: T, callback: EventCallback<T>) => void;
    off: <T extends AllEventTypes>(eventType: T, callback: EventCallback<T>) => void;
    getHistoricalEvents: typeof EnclaveSDK.prototype.getHistoricalEvents;
    // Event types for convenience
    EnclaveEventType: typeof EnclaveEventType;
    RegistryEventType: typeof RegistryEventType;
}

export const useEnclaveSDK = (config: UseEnclaveSDKConfig): UseEnclaveSDKReturn => {
    const [sdk, setSdk] = useState<EnclaveSDK | null>(null);
    const [isInitialized, setIsInitialized] = useState(false);
    const [isConnecting, setIsConnecting] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const sdkRef = useRef<EnclaveSDK | null>(null);

    const initializeSDK = useCallback(async (walletClient?: any) => {
        try {
            setError(null);

            const publicClient = createPublicClient({
                transport: http(config.rpcUrl)
            });

            const sdkConfig: SDKConfig = {
                publicClient,
                walletClient,
                contracts: config.contracts,
                chainId: config.chainId
            };

            const newSdk = new EnclaveSDK(sdkConfig);
            await newSdk.initialize();

            setSdk(newSdk);
            sdkRef.current = newSdk;
            setIsInitialized(true);
        } catch (err) {
            const errorMessage = err instanceof SDKError
                ? `SDK Error (${err.code}): ${err.message}`
                : `Failed to initialize SDK: ${err}`;
            setError(errorMessage);
            console.error('SDK initialization failed:', err);
        }
    }, [config.contracts, config.chainId, config.rpcUrl]);

    const connectWallet = useCallback(async () => {
        if (typeof window === 'undefined' || !window.ethereum) {
            setError('MetaMask not found. Please install MetaMask.');
            return;
        }

        try {
            setIsConnecting(true);
            setError(null);

            await window.ethereum.request({ method: 'eth_requestAccounts' });

            const walletClient = createWalletClient({
                transport: custom(window.ethereum)
            });

            await initializeSDK(walletClient);
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to connect wallet';
            setError(errorMessage);
            console.error('Wallet connection failed:', err);
        } finally {
            setIsConnecting(false);
        }
    }, [initializeSDK]);

    // Initialize SDK on mount
    useEffect(() => {
        if (config.autoConnect) {
            initializeSDK();
        }
    }, [config.autoConnect, initializeSDK]);

    // Cleanup on unmount
    useEffect(() => {
        return () => {
            if (sdkRef.current) {
                sdkRef.current.cleanup();
            }
        };
    }, []);

    // Contract interaction methods
    const requestE3 = useCallback((...args: Parameters<typeof EnclaveSDK.prototype.requestE3>) => {
        if (!sdk) throw new Error('SDK not initialized');
        return sdk.requestE3(...args);
    }, [sdk]);

    const activateE3 = useCallback((...args: Parameters<typeof EnclaveSDK.prototype.activateE3>) => {
        if (!sdk) throw new Error('SDK not initialized');
        return sdk.activateE3(...args);
    }, [sdk]);

    const publishInput = useCallback((...args: Parameters<typeof EnclaveSDK.prototype.publishInput>) => {
        if (!sdk) throw new Error('SDK not initialized');
        return sdk.publishInput(...args);
    }, [sdk]);

    const addCiphernode = useCallback((...args: Parameters<typeof EnclaveSDK.prototype.addCiphernode>) => {
        if (!sdk) throw new Error('SDK not initialized');
        return sdk.addCiphernode(...args);
    }, [sdk]);

    const removeCiphernode = useCallback((...args: Parameters<typeof EnclaveSDK.prototype.removeCiphernode>) => {
        if (!sdk) throw new Error('SDK not initialized');
        return sdk.removeCiphernode(...args);
    }, [sdk]);

    const getE3 = useCallback((...args: Parameters<typeof EnclaveSDK.prototype.getE3>) => {
        if (!sdk) throw new Error('SDK not initialized');
        return sdk.getE3(...args);
    }, [sdk]);

    const getCiphernode = useCallback((...args: Parameters<typeof EnclaveSDK.prototype.getCiphernode>) => {
        if (!sdk) throw new Error('SDK not initialized');
        return sdk.getCiphernode(...args);
    }, [sdk]);

    // Event handling methods
    const onEnclaveEvent = useCallback(<T extends AllEventTypes>(
        eventType: T,
        callback: EventCallback<T>
    ) => {
        if (!sdk) throw new Error('SDK not initialized');
        return sdk.onEnclaveEvent(eventType, callback);
    }, [sdk]);

    const off = useCallback(<T extends AllEventTypes>(
        eventType: T,
        callback: EventCallback<T>
    ) => {
        if (!sdk) throw new Error('SDK not initialized');
        return sdk.off(eventType, callback);
    }, [sdk]);

    const getHistoricalEvents = useCallback((...args: Parameters<typeof EnclaveSDK.prototype.getHistoricalEvents>) => {
        if (!sdk) throw new Error('SDK not initialized');
        return sdk.getHistoricalEvents(...args);
    }, [sdk]);

    return {
        sdk,
        isInitialized,
        isConnecting,
        error,
        connectWallet,
        requestE3,
        activateE3,
        publishInput,
        addCiphernode,
        removeCiphernode,
        getE3,
        getCiphernode,
        onEnclaveEvent,
        off,
        getHistoricalEvents,
        EnclaveEventType,
        RegistryEventType
    };
}; 