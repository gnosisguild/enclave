import { type PublicClient, type WalletClient, type Hash, type Log } from 'viem';
import { EventListener } from './event-listener';
import { ContractClient } from './contract-client';
import {
    type SDKConfig,
    type EventCallback,
    type EventListenerConfig,
    type AllEventTypes,
    type EnclaveEvent,
    EnclaveEventType,
    RegistryEventType
} from './types';
import {
    Enclave__factory,
    CiphernodeRegistryOwnable__factory
} from '../../types';
import { SDKError, isValidAddress } from './utils';

export class EnclaveSDK {
    private eventListener: EventListener;
    private contractClient: ContractClient;
    private initialized = false;

    constructor(private config: SDKConfig) {
        if (!config.publicClient) {
            throw new SDKError('Public client is required', 'MISSING_PUBLIC_CLIENT');
        }

        if (!isValidAddress(config.contracts.enclave)) {
            throw new SDKError('Invalid Enclave contract address', 'INVALID_ADDRESS');
        }

        if (!isValidAddress(config.contracts.ciphernodeRegistry)) {
            throw new SDKError('Invalid CiphernodeRegistry contract address', 'INVALID_ADDRESS');
        }

        this.eventListener = new EventListener(config.publicClient);
        this.contractClient = new ContractClient(
            config.publicClient,
            config.walletClient,
            config.contracts
        );
    }

    /**
     * Initialize the SDK
     */
    public async initialize(): Promise<void> {
        if (this.initialized) return;

        try {
            await this.contractClient.initialize();
            this.initialized = true;
        } catch (error) {
            throw new SDKError(
                `Failed to initialize SDK: ${error}`,
                'SDK_INITIALIZATION_FAILED'
            );
        }
    }

    /**
     * Request a new E3 computation
     */
    public async requestE3(params: {
        filter: `0x${string}`;
        threshold: [number, number];
        startWindow: [bigint, bigint];
        duration: bigint;
        e3Program: `0x${string}`;
        e3ProgramParams: `0x${string}`;
        computeProviderParams: `0x${string}`;
        value?: bigint;
        gasLimit?: bigint;
    }): Promise<Hash> {
        if (!this.initialized) {
            await this.initialize();
        }

        return this.contractClient.requestE3(
            params.filter,
            params.threshold,
            params.startWindow,
            params.duration,
            params.e3Program,
            params.e3ProgramParams,
            params.computeProviderParams,
            params.value,
            params.gasLimit
        );
    }

    /**
     * Activate an E3 computation
     */
    public async activateE3(
        e3Id: bigint,
        publicKey: `0x${string}`,
        gasLimit?: bigint
    ): Promise<Hash> {
        if (!this.initialized) {
            await this.initialize();
        }

        return this.contractClient.activateE3(e3Id, publicKey, gasLimit);
    }

    /**
     * Publish input for an E3 computation
     */
    public async publishInput(
        e3Id: bigint,
        data: `0x${string}`,
        gasLimit?: bigint
    ): Promise<Hash> {
        if (!this.initialized) {
            await this.initialize();
        }

        return this.contractClient.publishInput(e3Id, data, gasLimit);
    }

    /**
     * Add a ciphernode to the registry
     */
    public async addCiphernode(
        node: `0x${string}`,
        gasLimit?: bigint
    ): Promise<Hash> {
        if (!this.initialized) {
            await this.initialize();
        }

        return this.contractClient.addCiphernode(node, gasLimit);
    }

    /**
     * Remove a ciphernode from the registry
     */
    public async removeCiphernode(
        node: `0x${string}`,
        siblingNodes: bigint[],
        gasLimit?: bigint
    ): Promise<Hash> {
        if (!this.initialized) {
            await this.initialize();
        }

        return this.contractClient.removeCiphernode(node, siblingNodes, gasLimit);
    }

    /**
     * Get E3 information
     */
    public async getE3(e3Id: bigint): Promise<any> {
        if (!this.initialized) {
            await this.initialize();
        }

        return this.contractClient.getE3(e3Id);
    }

    /**
     * Get ciphernode information
     */
    public async getCiphernode(node: `0x${string}`): Promise<any> {
        if (!this.initialized) {
            await this.initialize();
        }

        return this.contractClient.getCiphernode(node);
    }

    /**
     * Unified Event Listening - Listen to any Enclave or Registry event
     */
    public onEnclaveEvent<T extends AllEventTypes>(
        eventType: T,
        callback: EventCallback<T>
    ): void {
        // Determine which contract to listen to based on event type
        const isEnclaveEvent = Object.values(EnclaveEventType).includes(eventType as EnclaveEventType);
        const contractAddress = isEnclaveEvent
            ? this.config.contracts.enclave
            : this.config.contracts.ciphernodeRegistry;
        const abi = isEnclaveEvent
            ? Enclave__factory.abi as any
            : CiphernodeRegistryOwnable__factory.abi as any;

        this.eventListener.watchContractEvent(
            contractAddress,
            eventType,
            abi,
            callback
        );
    }

    /**
     * Remove event listener
     */
    public off<T extends AllEventTypes>(eventType: T, callback: EventCallback<T>): void {
        this.eventListener.off(eventType, callback);
    }

    /**
     * Get historical events
     */
    public async getHistoricalEvents(
        eventType: AllEventTypes,
        fromBlock?: bigint,
        toBlock?: bigint
    ): Promise<Log[]> {
        const isEnclaveEvent = Object.values(EnclaveEventType).includes(eventType as EnclaveEventType);
        const contractAddress = isEnclaveEvent
            ? this.config.contracts.enclave
            : this.config.contracts.ciphernodeRegistry;
        const abi = isEnclaveEvent
            ? Enclave__factory.abi as any
            : CiphernodeRegistryOwnable__factory.abi as any;

        return this.eventListener.getHistoricalEvents(
            contractAddress,
            eventType,
            abi,
            fromBlock,
            toBlock
        );
    }

    /**
     * Start polling for events
     */
    public async startEventPolling(): Promise<void> {
        return this.eventListener.startPolling();
    }

    /**
     * Stop polling for events
     */
    public stopEventPolling(): void {
        this.eventListener.stopPolling();
    }

    /**
     * Utility methods
     */

    /**
     * Estimate gas for a transaction
     */
    public async estimateGas(
        functionName: string,
        args: readonly unknown[],
        contractAddress: `0x${string}`,
        abi: readonly unknown[],
        value?: bigint
    ): Promise<bigint> {
        return this.contractClient.estimateGas(functionName, args, contractAddress, abi, value);
    }

    /**
     * Wait for transaction confirmation
     */
    public async waitForTransaction(hash: Hash): Promise<any> {
        return this.contractClient.waitForTransaction(hash);
    }

    /**
     * Clean up resources
     */
    public cleanup(): void {
        this.eventListener.cleanup();
    }

    /**
     * Update SDK configuration
     */
    public updateConfig(newConfig: Partial<SDKConfig>): void {
        if (newConfig.publicClient) {
            this.config.publicClient = newConfig.publicClient;
            this.eventListener = new EventListener(newConfig.publicClient);
        }

        if (newConfig.walletClient) {
            this.config.walletClient = newConfig.walletClient;
        }

        if (newConfig.contracts) {
            this.config.contracts = { ...this.config.contracts, ...newConfig.contracts };
        }

        if (newConfig.chainId) {
            this.config.chainId = newConfig.chainId;
        }

        // Reinitialize contract client with new config
        this.contractClient = new ContractClient(
            this.config.publicClient,
            this.config.walletClient,
            this.config.contracts
        );

        this.initialized = false;
    }
} 