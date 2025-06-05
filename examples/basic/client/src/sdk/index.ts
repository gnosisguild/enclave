// Local SDK wrapper to handle CommonJS imports in Vite
import * as SDK from '@gnosis-guild/enclave/sdk';

// Re-export the main SDK class and its type
export const EnclaveSDK = SDK.EnclaveSDK;
export type EnclaveSDK = InstanceType<typeof SDK.EnclaveSDK>;

// Re-export types
export type {
    SDKConfig,
    EventListenerConfig,
    ContractInstances,
    EventFilter,
    EventCallback,
    SDKEventEmitter,
    AllEventTypes,
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
    EnclaveEventData,
    RegistryEventData
} from '@gnosis-guild/enclave/sdk';

// Re-export enums
export const EnclaveEventType = SDK.EnclaveEventType;
export const RegistryEventType = SDK.RegistryEventType;

// Re-export utilities
export const SDKError = SDK.SDKError;
export const isValidAddress = SDK.isValidAddress;
export const formatEventName = SDK.formatEventName;
export const generateEventId = SDK.generateEventId;
export const sleep = SDK.sleep;

// Re-export other classes
export const EventListener = SDK.EventListener;
export const ContractClient = SDK.ContractClient; 