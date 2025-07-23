// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import {
  type Abi,
  type Hash,
  type Log,
  WalletClient,
  createPublicClient,
  createWalletClient,
  http,
  webSocket,
} from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { hardhat, mainnet, monadTestnet, sepolia } from "viem/chains";

import {
  CiphernodeRegistryOwnable__factory,
  Enclave__factory,
} from "@gnosis-guild/enclave/types";
import { ContractClient } from "./contract-client";
import { EventListener } from "./event-listener";
import {
  type AllEventTypes,
  type E3,
  EnclaveEventType,
  type EventCallback,
  type SDKConfig,
} from "./types";
import { SDKError, isValidAddress } from "./utils";

export class EnclaveSDK {
  public static readonly chains = {
    1: mainnet,
    11155111: sepolia,
    41454: monadTestnet,
    31337: hardhat,
  } as const;

  private eventListener: EventListener;
  private contractClient: ContractClient;
  private initialized = false;

  constructor(private config: SDKConfig) {
    if (!config.publicClient) {
      throw new SDKError("Public client is required", "MISSING_PUBLIC_CLIENT");
    }

    if (!isValidAddress(config.contracts.enclave)) {
      throw new SDKError("Invalid Enclave contract address", "INVALID_ADDRESS");
    }

    if (!isValidAddress(config.contracts.ciphernodeRegistry)) {
      throw new SDKError(
        "Invalid CiphernodeRegistry contract address",
        "INVALID_ADDRESS",
      );
    }

    this.eventListener = new EventListener(config.publicClient);
    this.contractClient = new ContractClient(
      config.publicClient,
      config.walletClient,
      config.contracts,
    );
  }

  /**
   * Initialize the SDK
   */
  // TODO: Delete this it is redundant
  public async initialize(): Promise<void> {
    if (this.initialized) return;

    try {
      await this.contractClient.initialize();
      this.initialized = true;
    } catch (error) {
      throw new SDKError(
        `Failed to initialize SDK: ${error}`,
        "SDK_INITIALIZATION_FAILED",
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
    console.log(">>> REQUEST");

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
      params.gasLimit,
    );
  }

  /**
   * Activate an E3 computation
   */
  public async activateE3(
    e3Id: bigint,
    publicKey: `0x${string}`,
    gasLimit?: bigint,
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
    gasLimit?: bigint,
  ): Promise<Hash> {
    if (!this.initialized) {
      await this.initialize();
    }

    return this.contractClient.publishInput(e3Id, data, gasLimit);
  }

  /**
   * Publish ciphertext output for an E3 computation
   */
  public async publishCiphertextOutput(
    e3Id: bigint,
    ciphertextOutput: `0x${string}`,
    proof: `0x${string}`,
    gasLimit?: bigint,
  ): Promise<Hash> {
    if (!this.initialized) {
      await this.initialize();
    }

    return this.contractClient.publishCiphertextOutput(
      e3Id,
      ciphertextOutput,
      proof,
      gasLimit,
    );
  }

  /**
   * Get E3 information
   */
  public async getE3(e3Id: bigint): Promise<E3> {
    if (!this.initialized) {
      await this.initialize();
    }

    return this.contractClient.getE3(e3Id);
  }

  /**
   * Unified Event Listening - Listen to any Enclave or Registry event
   */
  public onEnclaveEvent<T extends AllEventTypes>(
    eventType: T,
    callback: EventCallback<T>,
  ): void {
    // Determine which contract to listen to based on event type
    const isEnclaveEvent = Object.values(EnclaveEventType).includes(
      eventType as EnclaveEventType,
    );
    const contractAddress = isEnclaveEvent
      ? this.config.contracts.enclave
      : this.config.contracts.ciphernodeRegistry;
    const abi = isEnclaveEvent
      ? Enclave__factory.abi
      : CiphernodeRegistryOwnable__factory.abi;

    void this.eventListener.watchContractEvent(
      contractAddress,
      eventType,
      abi,
      callback,
    );
  }

  /**
   * Remove event listener
   */
  public off<T extends AllEventTypes>(
    eventType: T,
    callback: EventCallback<T>,
  ): void {
    this.eventListener.off(eventType, callback);
  }

  /**
   * Handle an event only once
   */
  public once<T extends AllEventTypes>(
    type: T,
    callback: EventCallback<T>,
  ): void {
    const handler: EventCallback<T> = (event) => {
      this.off(type, handler);
      const prom = callback(event);
      if (prom) {
        prom.catch((e) => console.log(e));
      }
    };
    this.onEnclaveEvent(type, handler);
  }

  /**
   * Get historical events
   */
  public async getHistoricalEvents(
    eventType: AllEventTypes,
    fromBlock?: bigint,
    toBlock?: bigint,
  ): Promise<Log[]> {
    const isEnclaveEvent = Object.values(EnclaveEventType).includes(
      eventType as EnclaveEventType,
    );
    const contractAddress = isEnclaveEvent
      ? this.config.contracts.enclave
      : this.config.contracts.ciphernodeRegistry;
    const abi = isEnclaveEvent
      ? Enclave__factory.abi
      : CiphernodeRegistryOwnable__factory.abi;

    return this.eventListener.getHistoricalEvents(
      contractAddress,
      eventType,
      abi,
      fromBlock,
      toBlock,
    );
  }

  /**
   * Start polling for events
   */
  public async startEventPolling(): Promise<void> {
    void this.eventListener.startPolling();
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
    abi: Abi,
    value?: bigint,
  ): Promise<bigint> {
    return this.contractClient.estimateGas(
      functionName,
      args,
      contractAddress,
      abi,
      value,
    );
  }

  /**
   * Wait for transaction confirmation
   */
  public async waitForTransaction(hash: Hash): Promise<unknown> {
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
  // TODO: We should delete this as we don't want a stateful client.
  public updateConfig(newConfig: Partial<SDKConfig>): void {
    if (newConfig.publicClient) {
      this.config.publicClient = newConfig.publicClient;
      this.eventListener = new EventListener(newConfig.publicClient);
    }

    if (newConfig.walletClient) {
      this.config.walletClient = newConfig.walletClient;
    }

    if (newConfig.contracts) {
      this.config.contracts = {
        ...this.config.contracts,
        ...newConfig.contracts,
      };
    }

    if (newConfig.chainId) {
      this.config.chainId = newConfig.chainId;
    }

    this.contractClient = new ContractClient(
      this.config.publicClient,
      this.config.walletClient,
      this.config.contracts,
    );

    this.initialized = false;
  }

  public static create(options: {
    rpcUrl: string;
    contracts: {
      enclave: `0x${string}`;
      ciphernodeRegistry: `0x${string}`;
    };
    privateKey?: `0x${string}`;
    chainId: keyof typeof EnclaveSDK.chains;
  }): EnclaveSDK {
    const chain = EnclaveSDK.chains[options.chainId];

    const isWebSocket =
      options.rpcUrl.startsWith("ws://") || options.rpcUrl.startsWith("wss://");
    const transport = isWebSocket
      ? webSocket(options.rpcUrl, {
          keepAlive: { interval: 30_000 },
          reconnect: { attempts: 5, delay: 2_000 },
        })
      : http(options.rpcUrl);
    const publicClient = createPublicClient({
      chain,
      transport,
    }) as SDKConfig["publicClient"];
    let walletClient: WalletClient | undefined = undefined;
    if (options.privateKey) {
      const account = privateKeyToAccount(options.privateKey);
      walletClient = createWalletClient({
        account,
        chain,
        transport,
      });
    }

    return new EnclaveSDK({
      publicClient,
      walletClient,
      contracts: options.contracts,
    });
  }
}
