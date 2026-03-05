// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import type { Chain, PublicClient, WalletClient } from 'viem'
import type { ContractAddresses } from './contracts/types'
import type { ThresholdBfvParamsPresetName } from './encryption/types'

// Re-export all sub-module types for backward compatibility
export type { BfvParams, ThresholdBfvParamsPresetName, VerifiableEncryptionResult, EncryptedValueAndPublicInputs } from './encryption/types'

export { ThresholdBfvParamsPresetNames } from './encryption/types'

export type { ContractAddresses, E3, E3RequestParams } from './contracts/types'
export { E3Stage, FailureReason } from './contracts/types'

export { EnclaveEventType, RegistryEventType } from './events/types'

export type {
  AllEventTypes,
  EnclaveEvent,
  EventCallback,
  EventFilter,
  SDKEventEmitter,
  EventListenerConfig,
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
} from './events/types'

export interface SDKConfig {
  publicClient: PublicClient
  walletClient?: WalletClient
  contracts: ContractAddresses
  chain?: Chain
  thresholdBfvParamsPresetName: ThresholdBfvParamsPresetName
}
