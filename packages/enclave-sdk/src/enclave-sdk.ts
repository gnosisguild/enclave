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
import initializeWasm from "@enclave-e3/wasm/init";

import {
  CiphernodeRegistryOwnable__factory,
  Enclave__factory,
} from "@enclave-e3/contracts/types";
import { ContractClient } from "./contract-client";
import { EventListener } from "./event-listener";
import { FheProtocol, EnclaveEventType, BfvProtocolParams } from "./types";
import { SDKError, isValidAddress } from "./utils";

import type {
  AllEventTypes,
  E3,
  EventCallback,
  SDKConfig,
  ProtocolParams,
  VerifiableEncryptionResult,
} from "./types";
import {
  bfv_encrypt_number,
  bfv_verifiable_encrypt_number,
} from "@enclave-e3/wasm";
import { CircuitInputs, generateProof } from "./greco";
import { CompiledCircuit } from "@noir-lang/noir_js";

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
  private protocol: FheProtocol;
  private protocolParams: ProtocolParams;

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
        "INVALID_ADDRESS"
      );
    }

    this.eventListener = new EventListener(config.publicClient);
    this.contractClient = new ContractClient(
      config.publicClient,
      config.walletClient,
      config.contracts
    );

    this.protocol = config.protocol;

    if (config.protocolParams) {
      this.protocolParams = config.protocolParams;
    } else {
      switch (this.protocol) {
        case FheProtocol.BFV:
          this.protocolParams = BfvProtocolParams.BFV_NORMAL;
          break;
        default:
          throw new Error("Protocol not supported");
      }
    }
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
        "SDK_INITIALIZATION_FAILED"
      );
    }
  }

  /**
   * Encrypt a number using the configured protocol
   * @param data - The number to encrypt
   * @param publicKey - The public key to use for encryption
   * @returns The encrypted number
   */
  public async encryptNumber(
    data: bigint,
    publicKey: Uint8Array
  ): Promise<Uint8Array> {
    await initializeWasm();
    switch (this.protocol) {
      case FheProtocol.BFV:
        return bfv_encrypt_number(
          data,
          publicKey,
          this.protocolParams.degree,
          this.protocolParams.plaintextModulus,
          this.protocolParams.moduli
        );
      default:
        throw new Error("Protocol not supported");
    }
  }

  /**
   * Encrypt a number using the configured protocol and generate a zk-SNARK proof using Greco
   * @param data - The number to encrypt
   * @param publicKey - The public key to use for encryption
   * @param circuit - The circuit to use for proof generation
   * @returns The encrypted number and the proof
   */
  public async encryptNumberAndGenProof(
    data: bigint,
    publicKey: Uint8Array,
    circuit: CompiledCircuit
  ): Promise<VerifiableEncryptionResult> {
    await initializeWasm();
    switch (this.protocol) {
      case FheProtocol.BFV:
        const [encryptedVote, circuitInputs] = bfv_verifiable_encrypt_number(
          data,
          publicKey,
          this.protocolParams.degree,
          this.protocolParams.plaintextModulus,
          this.protocolParams.moduli
        );

        const inputs = JSON.parse(circuitInputs) as CircuitInputs;
        // inputs.params = defaultParams;
        const proof = await generateProof(inputs, circuit);

        return {
          encryptedVote,
          proof,
        };
      default:
        throw new Error("Protocol not supported");
    }
  }

  /**
   * Request a new E3 computation
   */
  public async requestE3(params: {
    threshold: [number, number];
    startWindow: [bigint, bigint];
    duration: bigint;
    e3Program: `0x${string}`;
    e3ProgramParams: `0x${string}`;
    computeProviderParams: `0x${string}`;
    customParams?: `0x${string}`;
    gasLimit?: bigint;
  }): Promise<Hash> {
    console.log(">>> REQUEST");

    if (!this.initialized) {
      await this.initialize();
    }

    return this.contractClient.requestE3(
      params.threshold,
      params.startWindow,
      params.duration,
      params.e3Program,
      params.e3ProgramParams,
      params.computeProviderParams,
      params.customParams,
      params.gasLimit
    );
  }

  /**
   * Get the public key for an E3 computation
   * @param e3Id - The ID of the E3 computation
   * @returns The public key
   */
  public async getE3PublicKey(e3Id: bigint): Promise<`0x${string}`> {
    if (!this.initialized) {
      await this.initialize();
    }

    return this.contractClient.getE3PublicKey(e3Id);
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
   * Publish ciphertext output for an E3 computation
   */
  public async publishCiphertextOutput(
    e3Id: bigint,
    ciphertextOutput: `0x${string}`,
    proof: `0x${string}`,
    gasLimit?: bigint
  ): Promise<Hash> {
    if (!this.initialized) {
      await this.initialize();
    }

    return this.contractClient.publishCiphertextOutput(
      e3Id,
      ciphertextOutput,
      proof,
      gasLimit
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
    callback: EventCallback<T>
  ): void {
    // Determine which contract to listen to based on event type
    const isEnclaveEvent = Object.values(EnclaveEventType).includes(
      eventType as EnclaveEventType
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
      callback
    );
  }

  /**
   * Remove event listener
   */
  public off<T extends AllEventTypes>(
    eventType: T,
    callback: EventCallback<T>
  ): void {
    this.eventListener.off(eventType, callback);
  }

  /**
   * Handle an event only once
   */
  public once<T extends AllEventTypes>(
    type: T,
    callback: EventCallback<T>
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
    toBlock?: bigint
  ): Promise<Log[]> {
    const isEnclaveEvent = Object.values(EnclaveEventType).includes(
      eventType as EnclaveEventType
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
      toBlock
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
    value?: bigint
  ): Promise<bigint> {
    return this.contractClient.estimateGas(
      functionName,
      args,
      contractAddress,
      abi,
      value
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
      this.config.contracts
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
    protocol: FheProtocol;
    protocolParams?: ProtocolParams;
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
      chainId: options.chainId,
      protocol: options.protocol,
      protocolParams: options.protocolParams,
    });
  }
}
