/**
 * @gnosis-guild/enclave-react
 *
 * React hooks and utilities for Enclave SDK
 */

export { useEnclaveSDK } from "./useEnclaveSDK";
export type { UseEnclaveSDKConfig, UseEnclaveSDKReturn } from "./useEnclaveSDK";

// Re-export commonly used types from the main SDK for convenience
export type {
  AllEventTypes,
  EventCallback,
  EnclaveEvent,
  E3RequestedData,
  E3ActivatedData,
  InputPublishedData,
  CiphertextOutputPublishedData,
  PlaintextOutputPublishedData,
  CiphernodeAddedData,
  CiphernodeRemovedData,
  CommitteeRequestedData,
  CommitteePublishedData,
} from "@gnosis-guild/enclave-sdk";

export { EnclaveEventType, RegistryEventType } from "@gnosis-guild/enclave-sdk";
