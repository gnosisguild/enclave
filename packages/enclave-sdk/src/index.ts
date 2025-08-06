// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Main SDK class
export { EnclaveSDK } from "./enclave-sdk";

// Core classes
export { EventListener } from "./event-listener";
export { ContractClient } from "./contract-client";

// Types and interfaces
export type {
  E3,
  SDKConfig,
  EventListenerConfig,
  ContractInstances,
  EventFilter,
  EventCallback,
  SDKEventEmitter,
  AllEventTypes,
  EnclaveEvent,
  // Event data types
  E3RequestedData,
  E3ActivatedData,
  InputPublishedData,
  CiphertextOutputPublishedData,
  PlaintextOutputPublishedData,
  CiphernodeAddedData,
  CiphernodeRemovedData,
  CommitteeRequestedData,
  CommitteePublishedData,
  EnclaveEventData,
  RegistryEventData,
  ProtocolParams,
  VerifiableEncryptionResult,
} from "./types";

// enums and constants
export { EnclaveEventType, RegistryEventType, FheProtocol, BfvProtocolParams } from "./types";

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
  // BFV and E3 utilities
  BFV_PARAMS_SET,
  DEFAULT_COMPUTE_PROVIDER_PARAMS,
  DEFAULT_E3_CONFIG,
  encodeBfvParams,
  encodeComputeProviderParams,
  calculateStartWindow,
  decodePlaintextOutput,
  type ComputeProviderParams,
} from "./utils";

export { generateProof } from "./greco";
