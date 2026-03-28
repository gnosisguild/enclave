// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import type { Log } from 'viem'

export enum EnclaveEventType {
  E3_REQUESTED = 'E3Requested',
  CIPHERTEXT_OUTPUT_PUBLISHED = 'CiphertextOutputPublished',
  PLAINTEXT_OUTPUT_PUBLISHED = 'PlaintextOutputPublished',
  E3_PROGRAM_ENABLED = 'E3ProgramEnabled',
  E3_PROGRAM_DISABLED = 'E3ProgramDisabled',
  ENCRYPTION_SCHEME_ENABLED = 'EncryptionSchemeEnabled',
  ENCRYPTION_SCHEME_DISABLED = 'EncryptionSchemeDisabled',
  CIPHERNODE_REGISTRY_SET = 'CiphernodeRegistrySet',
  MAX_DURATION_SET = 'MaxDurationSet',
  ALLOWED_E3_PROGRAMS_PARAMS_SET = 'AllowedE3ProgramsParamsSet',
  OWNERSHIP_TRANSFERRED = 'OwnershipTransferred',
  INITIALIZED = 'Initialized',
}

export enum RegistryEventType {
  COMMITTEE_REQUESTED = 'CommitteeRequested',
  COMMITTEE_PUBLISHED = 'CommitteePublished',
  COMMITTEE_FINALIZED = 'CommitteeFinalized',
  ENCLAVE_SET = 'EnclaveSet',
  OWNERSHIP_TRANSFERRED = 'OwnershipTransferred',
  INITIALIZED = 'Initialized',
}

export type AllEventTypes = EnclaveEventType | RegistryEventType

export interface E3RequestedData {
  e3Id: bigint
  e3: {
    seed: bigint
    committeeSize: number
    requestBlock: bigint
    inputWindow: readonly [bigint, bigint]
    encryptionSchemeId: string
    e3Program: string
    e3ProgramParams: string
    decryptionVerifier: string
    committeePublicKey: string
    ciphertextOutput: string
    plaintextOutput: string
  }
  filter: string
  e3Program: string
}

export interface E3ActivatedData {
  e3Id: bigint
  expiration: bigint
  committeePublicKey: string
}

export interface CiphertextOutputPublishedData {
  e3Id: bigint
  ciphertextOutput: string
}

export interface PlaintextOutputPublishedData {
  e3Id: bigint
  plaintextOutput: string
  proof: string
}

export interface CiphernodeAddedData {
  node: string
  index: bigint
  numNodes: bigint
  size: bigint
}

export interface CiphernodeRemovedData {
  node: string
  index: bigint
  numNodes: bigint
  size: bigint
}

export interface CommitteeRequestedData {
  e3Id: bigint
  seed: bigint
  threshold: [bigint, bigint]
  requestBlock: bigint
  committeeDeadline: bigint
}

export interface CommitteePublishedData {
  e3Id: bigint
  nodes: string[]
  publicKey: string
  proof: string
}

export interface CommitteeFinalizedData {
  e3Id: bigint
  nodes: string[]
  scores: bigint[]
}

export interface EnclaveEventData {
  [EnclaveEventType.E3_REQUESTED]: E3RequestedData
  [EnclaveEventType.CIPHERTEXT_OUTPUT_PUBLISHED]: CiphertextOutputPublishedData
  [EnclaveEventType.PLAINTEXT_OUTPUT_PUBLISHED]: PlaintextOutputPublishedData
  [EnclaveEventType.E3_PROGRAM_ENABLED]: { e3Program: string }
  [EnclaveEventType.E3_PROGRAM_DISABLED]: { e3Program: string }
  [EnclaveEventType.ENCRYPTION_SCHEME_ENABLED]: { encryptionSchemeId: string }
  [EnclaveEventType.ENCRYPTION_SCHEME_DISABLED]: { encryptionSchemeId: string }
  [EnclaveEventType.CIPHERNODE_REGISTRY_SET]: { ciphernodeRegistry: string }
  [EnclaveEventType.MAX_DURATION_SET]: { maxDuration: bigint }
  [EnclaveEventType.ALLOWED_E3_PROGRAMS_PARAMS_SET]: { e3ProgramParams: string[] }
  [EnclaveEventType.OWNERSHIP_TRANSFERRED]: { previousOwner: string; newOwner: string }
  [EnclaveEventType.INITIALIZED]: { version: bigint }
}

export interface RegistryEventData {
  [RegistryEventType.COMMITTEE_REQUESTED]: CommitteeRequestedData
  [RegistryEventType.COMMITTEE_PUBLISHED]: CommitteePublishedData
  [RegistryEventType.COMMITTEE_FINALIZED]: CommitteeFinalizedData
  [RegistryEventType.ENCLAVE_SET]: { enclave: string }
  [RegistryEventType.OWNERSHIP_TRANSFERRED]: { previousOwner: string; newOwner: string }
  [RegistryEventType.INITIALIZED]: { version: bigint }
}

export interface EnclaveEvent<T extends AllEventTypes> {
  type: T
  data: T extends EnclaveEventType ? EnclaveEventData[T] : T extends RegistryEventType ? RegistryEventData[T] : unknown
  log: Log
  timestamp: Date
  blockNumber: bigint
  transactionHash: string
}

export type EventCallback<T extends AllEventTypes = AllEventTypes> = (event: EnclaveEvent<T>) => void | Promise<void>

export interface EventFilter<T = unknown> {
  address?: `0x${string}`
  fromBlock?: bigint
  toBlock?: bigint
  args?: Partial<T>
}

export interface SDKEventEmitter {
  on<T extends AllEventTypes>(eventType: T, callback: EventCallback<T>): void
  off<T extends AllEventTypes>(eventType: T, callback: EventCallback<T>): void
  emit<T extends AllEventTypes>(event: EnclaveEvent<T>): void
}

export interface EventListenerConfig {
  fromBlock?: bigint
  toBlock?: bigint
  polling?: boolean
  pollingInterval?: number
}
