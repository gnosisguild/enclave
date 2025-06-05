import {
    type PublicClient,
    type WalletClient,
    type Hash,
} from 'viem';
import {
    Enclave__factory,
    CiphernodeRegistryOwnable__factory,
} from '../../types';
import { SDKError, isValidAddress } from './utils';

export class ContractClient {
    private contractInfo: {
        enclave: { address: `0x${string}`; abi: any };
        ciphernodeRegistry: { address: `0x${string}`; abi: any };
    } | null = null;

    constructor(
        private publicClient: PublicClient,
        private walletClient?: WalletClient,
        private addresses: {
            enclave: `0x${string}`;
            ciphernodeRegistry: `0x${string}`;
        } = {
                enclave: '0x0000000000000000000000000000000000000000',
                ciphernodeRegistry: '0x0000000000000000000000000000000000000000'
            }
    ) {
        if (!isValidAddress(addresses.enclave)) {
            throw new SDKError('Invalid Enclave contract address', 'INVALID_ADDRESS');
        }
        if (!isValidAddress(addresses.ciphernodeRegistry)) {
            throw new SDKError('Invalid CiphernodeRegistry contract address', 'INVALID_ADDRESS');
        }
    }

    /**
     * Initialize contract instances
     */
    public async initialize(): Promise<void> {
        try {
            this.contractInfo = {
                enclave: {
                    address: this.addresses.enclave,
                    abi: Enclave__factory.abi
                },
                ciphernodeRegistry: {
                    address: this.addresses.ciphernodeRegistry,
                    abi: CiphernodeRegistryOwnable__factory.abi
                }
            };
        } catch (error) {
            throw new SDKError(
                `Failed to initialize contracts: ${error}`,
                'INITIALIZATION_FAILED'
            );
        }
    }

    /**
     * Request a new E3 computation
     * request(address filter, uint32[2] threshold, uint256[2] startWindow, uint256 duration, IE3Program e3Program, bytes e3ProgramParams, bytes computeProviderParams)
     */
    public async requestE3(
        filter: `0x${string}`,
        threshold: [number, number],
        startWindow: [bigint, bigint],
        duration: bigint,
        e3Program: `0x${string}`,
        e3ProgramParams: `0x${string}`,
        computeProviderParams: `0x${string}`,
        value?: bigint,
        gasLimit?: bigint
    ): Promise<Hash> {
        if (!this.walletClient) {
            throw new SDKError('Wallet client required for write operations', 'NO_WALLET');
        }

        if (!this.contractInfo) {
            await this.initialize();
        }

        try {
            const account = this.walletClient.account;
            if (!account) {
                throw new SDKError('No account connected', 'NO_ACCOUNT');
            }

            // Simulate transaction
            const { request } = await this.publicClient.simulateContract({
                address: this.addresses.enclave,
                abi: Enclave__factory.abi,
                functionName: 'request',
                args: [filter, threshold, startWindow, duration, e3Program, e3ProgramParams, computeProviderParams],
                account,
                value: value || BigInt(0),
                gas: gasLimit
            });

            // Execute transaction
            const hash = await this.walletClient.writeContract(request);

            return hash;
        } catch (error) {
            throw new SDKError(
                `Failed to request E3: ${error}`,
                'REQUEST_E3_FAILED'
            );
        }
    }

    /**
     * Activate an E3 computation
     * activate(uint256 e3Id, bytes memory publicKey)
     */
    public async activateE3(
        e3Id: bigint,
        publicKey: `0x${string}`,
        gasLimit?: bigint
    ): Promise<Hash> {
        if (!this.walletClient) {
            throw new SDKError('Wallet client required for write operations', 'NO_WALLET');
        }

        if (!this.contractInfo) {
            await this.initialize();
        }

        try {
            const account = this.walletClient.account;
            if (!account) {
                throw new SDKError('No account connected', 'NO_ACCOUNT');
            }

            const { request } = await this.publicClient.simulateContract({
                address: this.addresses.enclave,
                abi: Enclave__factory.abi,
                functionName: 'activate',
                args: [e3Id, publicKey],
                account,
                gas: gasLimit
            });

            const hash = await this.walletClient.writeContract(request);

            return hash;
        } catch (error) {
            throw new SDKError(
                `Failed to activate E3: ${error}`,
                'ACTIVATE_E3_FAILED'
            );
        }
    }

    /**
     * Publish input for an E3 computation
     * publishInput(uint256 e3Id, bytes memory data)
     */
    public async publishInput(
        e3Id: bigint,
        data: `0x${string}`,
        gasLimit?: bigint
    ): Promise<Hash> {
        if (!this.walletClient) {
            throw new SDKError('Wallet client required for write operations', 'NO_WALLET');
        }

        if (!this.contractInfo) {
            await this.initialize();
        }

        try {
            const account = this.walletClient.account;
            if (!account) {
                throw new SDKError('No account connected', 'NO_ACCOUNT');
            }

            const { request } = await this.publicClient.simulateContract({
                address: this.addresses.enclave,
                abi: Enclave__factory.abi,
                functionName: 'publishInput',
                args: [e3Id, data],
                account,
                gas: gasLimit
            });

            const hash = await this.walletClient.writeContract(request);

            return hash;
        } catch (error) {
            throw new SDKError(
                `Failed to publish input: ${error}`,
                'PUBLISH_INPUT_FAILED'
            );
        }
    }

    /**
     * Get E3 information
     * Based on the contract: getE3(uint256 e3Id) returns (E3 memory e3)
     */
    public async getE3(e3Id: bigint): Promise<any> {
        if (!this.contractInfo) {
            await this.initialize();
        }

        try {
            const result = await this.publicClient.readContract({
                address: this.addresses.enclave,
                abi: Enclave__factory.abi,
                functionName: 'getE3',
                args: [e3Id]
            });

            return result;
        } catch (error) {
            throw new SDKError(
                `Failed to get E3: ${error}`,
                'GET_E3_FAILED'
            );
        }
    }

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
        if (!this.walletClient) {
            throw new SDKError('Wallet client required for gas estimation', 'NO_WALLET');
        }

        try {
            const account = this.walletClient.account;
            if (!account) {
                throw new SDKError('No account connected', 'NO_ACCOUNT');
            }

            const estimateParams: any = {
                address: contractAddress,
                abi,
                functionName,
                args,
                account,
            };

            if (value !== undefined) {
                estimateParams.value = value;
            }

            const gas = await this.publicClient.estimateContractGas(estimateParams);

            return gas;
        } catch (error) {
            throw new SDKError(
                `Failed to estimate gas: ${error}`,
                'GAS_ESTIMATION_FAILED'
            );
        }
    }

    /**
     * Wait for transaction confirmation
     */
    public async waitForTransaction(hash: Hash): Promise<any> {
        try {
            const receipt = await this.publicClient.waitForTransactionReceipt({
                hash,
                confirmations: 1
            });

            return receipt;
        } catch (error) {
            throw new SDKError(
                `Failed to wait for transaction: ${error}`,
                'TRANSACTION_WAIT_FAILED'
            );
        }
    }
} 