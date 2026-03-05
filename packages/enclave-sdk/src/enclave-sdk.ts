// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import {
  type Abi,
  type Chain,
  type Hash,
  type Log,
  type PublicClient,
  type WalletClient,
  createPublicClient,
  createWalletClient,
  http,
  webSocket,
} from 'viem'
import { privateKeyToAccount } from 'viem/accounts'

import { ContractClient } from './contracts/contract-client'
import { EventListener } from './events/event-listener'
import {
  getThresholdBfvParamsSet,
  generatePublicKey,
  computePublicKeyCommitment,
  encryptNumber,
  encryptVector,
  encryptNumberAndGenInputs,
  encryptNumberAndGenProof,
  encryptVectorAndGenInputs,
  encryptVectorAndGenProof,
} from './encryption/encrypt'
import { ThresholdBfvParamsPresetNames } from './encryption/types'
import { SDKError, isValidAddress } from './utils'

import type { SDKConfig } from './types'
import type { AllEventTypes, EventCallback } from './events/types'
import type { E3 } from './contracts/types'
import type { BfvParams, EncryptedValueAndPublicInputs, ThresholdBfvParamsPresetName, VerifiableEncryptionResult } from './encryption/types'

export class EnclaveSDK {
  private eventListener: EventListener
  private contractClient: ContractClient
  private thresholdBfvParamsPresetName: ThresholdBfvParamsPresetName
  private publicClient: PublicClient

  constructor(private config: SDKConfig) {
    if (!config.publicClient) {
      throw new SDKError('Public client is required', 'MISSING_PUBLIC_CLIENT')
    }

    if (!isValidAddress(config.contracts.enclave)) {
      throw new SDKError('Invalid Enclave contract address', 'INVALID_ADDRESS')
    }

    if (!isValidAddress(config.contracts.ciphernodeRegistry)) {
      throw new SDKError('Invalid CiphernodeRegistry contract address', 'INVALID_ADDRESS')
    }

    if (!isValidAddress(config.contracts.feeToken)) {
      throw new SDKError('Invalid FeeToken contract address', 'INVALID_ADDRESS')
    }

    if (!config.thresholdBfvParamsPresetName) {
      throw new SDKError('Threshold BFV parameters preset name is required', 'MISSING_THRESHOLD_BFV_PARAMS_PRESET_NAME')
    }

    if (!Object.values(ThresholdBfvParamsPresetNames).includes(config.thresholdBfvParamsPresetName)) {
      throw new SDKError(
        `Invalid threshold BFV parameters preset name: ${config.thresholdBfvParamsPresetName}`,
        'INVALID_THRESHOLD_BFV_PARAMS_PRESET_NAME',
      )
    }

    this.thresholdBfvParamsPresetName = config.thresholdBfvParamsPresetName
    this.publicClient = config.publicClient

    this.contractClient = new ContractClient({
      publicClient: config.publicClient,
      walletClient: config.walletClient,
      contracts: config.contracts,
    })

    this.eventListener = new EventListener({
      publicClient: config.publicClient,
      contracts: config.contracts,
    })
  }

  // --- Encryption (delegates to standalone functions) ---

  public getPublicClient(): PublicClient {
    return this.publicClient
  }

  public async getThresholdBfvParamsSet(): Promise<BfvParams> {
    return getThresholdBfvParamsSet(this.thresholdBfvParamsPresetName)
  }

  public async generatePublicKey(): Promise<Uint8Array> {
    return generatePublicKey(this.thresholdBfvParamsPresetName)
  }

  public async computePublicKeyCommitment(publicKey: Uint8Array): Promise<Uint8Array> {
    return computePublicKeyCommitment(publicKey, this.thresholdBfvParamsPresetName)
  }

  public async encryptNumber(data: bigint, publicKey: Uint8Array): Promise<Uint8Array> {
    return encryptNumber(data, publicKey, this.thresholdBfvParamsPresetName)
  }

  public async encryptVector(data: BigUint64Array, publicKey: Uint8Array): Promise<Uint8Array> {
    return encryptVector(data, publicKey, this.thresholdBfvParamsPresetName)
  }

  public async encryptNumberAndGenInputs(data: bigint, publicKey: Uint8Array): Promise<EncryptedValueAndPublicInputs> {
    return encryptNumberAndGenInputs(data, publicKey, this.thresholdBfvParamsPresetName)
  }

  public async encryptNumberAndGenProof(data: bigint, publicKey: Uint8Array): Promise<VerifiableEncryptionResult> {
    return encryptNumberAndGenProof(data, publicKey, this.thresholdBfvParamsPresetName)
  }

  public async encryptVectorAndGenInputs(data: BigUint64Array, publicKey: Uint8Array): Promise<EncryptedValueAndPublicInputs> {
    return encryptVectorAndGenInputs(data, publicKey, this.thresholdBfvParamsPresetName)
  }

  public async encryptVectorAndGenProof(data: BigUint64Array, publicKey: Uint8Array): Promise<VerifiableEncryptionResult> {
    return encryptVectorAndGenProof(data, publicKey, this.thresholdBfvParamsPresetName)
  }

  // --- Contracts (delegates to ContractClient) ---

  public async approveFeeToken(amount: bigint): Promise<Hash> {
    return this.contractClient.approveFeeToken(amount)
  }

  public async requestE3(params: {
    threshold: [number, number]
    inputWindow: [bigint, bigint]
    e3Program: `0x${string}`
    e3ProgramParams: `0x${string}`
    computeProviderParams: `0x${string}`
    customParams?: `0x${string}`
    gasLimit?: bigint
  }): Promise<Hash> {
    return this.contractClient.requestE3(params)
  }

  public async getE3PublicKey(e3Id: bigint): Promise<`0x${string}`> {
    return this.contractClient.getE3PublicKey(e3Id)
  }

  public async publishCiphertextOutput(
    e3Id: bigint,
    ciphertextOutput: `0x${string}`,
    proof: `0x${string}`,
    gasLimit?: bigint,
  ): Promise<Hash> {
    return this.contractClient.publishCiphertextOutput(e3Id, ciphertextOutput, proof, gasLimit)
  }

  public async getE3(e3Id: bigint): Promise<E3> {
    return this.contractClient.getE3(e3Id)
  }

  public async estimateGas(
    functionName: string,
    args: readonly unknown[],
    contractAddress: `0x${string}`,
    abi: Abi,
    value?: bigint,
  ): Promise<bigint> {
    return this.contractClient.estimateGas(functionName, args, contractAddress, abi, value)
  }

  public async waitForTransaction(hash: Hash): Promise<unknown> {
    return this.contractClient.waitForTransaction(hash)
  }

  // --- Events (delegates to EventListener) ---

  public onEnclaveEvent<T extends AllEventTypes>(eventType: T, callback: EventCallback<T>): void {
    this.eventListener.onEnclaveEvent(eventType, callback)
  }

  public off<T extends AllEventTypes>(eventType: T, callback: EventCallback<T>): void {
    this.eventListener.off(eventType, callback)
  }

  public once<T extends AllEventTypes>(type: T, callback: EventCallback<T>): void {
    this.eventListener.once(type, callback)
  }

  public async getHistoricalEvents(eventType: AllEventTypes, fromBlock?: bigint, toBlock?: bigint): Promise<Log[]> {
    return this.eventListener.getHistoricalEvents(eventType, fromBlock, toBlock)
  }

  public async startEventPolling(): Promise<void> {
    void this.eventListener.startPolling()
  }

  public stopEventPolling(): void {
    this.eventListener.stopPolling()
  }

  public cleanup(): void {
    this.eventListener.cleanup()
  }

  // --- Factory ---

  public static create(options: {
    rpcUrl: string
    contracts: {
      enclave: `0x${string}`
      ciphernodeRegistry: `0x${string}`
      feeToken: `0x${string}`
    }
    privateKey?: `0x${string}`
    chain: Chain
    thresholdBfvParamsPresetName: ThresholdBfvParamsPresetName
  }): EnclaveSDK {
    const isWebSocket = options.rpcUrl.startsWith('ws://') || options.rpcUrl.startsWith('wss://')
    const transport = isWebSocket
      ? webSocket(options.rpcUrl, {
          keepAlive: { interval: 30_000 },
          reconnect: { attempts: 5, delay: 2_000 },
        })
      : http(options.rpcUrl)

    const publicClient = createPublicClient({
      chain: options.chain,
      transport,
    }) as SDKConfig['publicClient']

    let walletClient: WalletClient | undefined
    if (options.privateKey) {
      const account = privateKeyToAccount(options.privateKey)
      walletClient = createWalletClient({
        account,
        chain: options.chain,
        transport,
      })
    }

    return new EnclaveSDK({
      publicClient,
      walletClient,
      contracts: options.contracts,
      chain: options.chain,
      thresholdBfvParamsPresetName: options.thresholdBfvParamsPresetName,
    })
  }
}
