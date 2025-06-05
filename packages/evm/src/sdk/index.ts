// Main SDK class
export { EnclaveSDK } from './enclave-sdk';

// Core classes
export { EventListener } from './event-listener';
export { ContractClient } from './contract-client';

// Types and interfaces
export type {
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
    RegistryEventData
} from './types';

// Event enums
export { EnclaveEventType, RegistryEventType } from './types';

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
    type ComputeProviderParams
} from './utils'; 