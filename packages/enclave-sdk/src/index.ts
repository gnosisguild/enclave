// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Main SDK class
export { EnclaveSDK } from './enclave-sdk'

// Core classes
export { EventListener } from './events/event-listener'
export { ContractClient } from './contracts/contract-client'
export type { ContractClientConfig } from './contracts/contract-client'
export type { EventListenerOptions } from './events/event-listener'

// Standalone encryption functions
export {
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

// Types and interfaces (re-exported from sub-modules via types.ts)
export type {
  SDKConfig,
  ContractAddresses,
  E3,
  E3RequestParams,
  EventListenerConfig,
  EventFilter,
  EventCallback,
  SDKEventEmitter,
  AllEventTypes,
  EnclaveEvent,
  E3RequestedData,
  E3ActivatedData,
  CiphertextOutputPublishedData,
  PlaintextOutputPublishedData,
  CiphernodeAddedData,
  CiphernodeRemovedData,
  CommitteeRequestedData,
  CommitteePublishedData,
  CommitteeFinalizedData,
  EnclaveEventData,
  RegistryEventData,
  BfvParams,
  VerifiableEncryptionResult,
  EncryptedValueAndPublicInputs,
  ThresholdBfvParamsPresetName,
} from './types'

// Enums and constants
export { EnclaveEventType, RegistryEventType, ThresholdBfvParamsPresetNames, E3Stage, FailureReason } from './types'

// Export utilities
export {
  SDKError,
  isValidAddress,
  isValidHash,
  formatEventName,
  parseEventData,
  formatBigInt,
  parseBigInt,
  generateEventId,
  sleep,
  getCurrentTimestamp,
  DEFAULT_COMPUTE_PROVIDER_PARAMS,
  DEFAULT_E3_CONFIG,
  encodeBfvParams,
  encodeComputeProviderParams,
  encodeCustomParams,
  calculateInputWindow,
  decodePlaintextOutput,
  type ComputeProviderParams,
} from './utils'

export { generateProof } from './greco'
