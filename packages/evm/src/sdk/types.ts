import { type Log } from "viem";
import { type PublicClient, type WalletClient } from "viem";

import {
  type CiphernodeRegistryOwnable,
  type Enclave,
  type MockCiphernodeRegistry,
} from "../../types";

export interface SDKConfig {
  publicClient: PublicClient;
  walletClient?: WalletClient;
  contracts: {
    enclave: `0x${string}`;
    ciphernodeRegistry: `0x${string}`;
  };
  chainId?: number;
}

export interface EventListenerConfig {
  fromBlock?: bigint;
  toBlock?: bigint;
  polling?: boolean;
  pollingInterval?: number;
}

export interface ContractInstances {
  enclave: Enclave;
  ciphernodeRegistry: CiphernodeRegistryOwnable | MockCiphernodeRegistry;
}

// Unified Event System
export enum EnclaveEventType {
  // E3 Lifecycle Events
  E3_REQUESTED = "E3Requested",
  E3_ACTIVATED = "E3Activated",
  INPUT_PUBLISHED = "InputPublished",
  CIPHERTEXT_OUTPUT_PUBLISHED = "CiphertextOutputPublished",
  PLAINTEXT_OUTPUT_PUBLISHED = "PlaintextOutputPublished",

  // E3 Program Management
  E3_PROGRAM_ENABLED = "E3ProgramEnabled",
  E3_PROGRAM_DISABLED = "E3ProgramDisabled",

  // Encryption Scheme Management
  ENCRYPTION_SCHEME_ENABLED = "EncryptionSchemeEnabled",
  ENCRYPTION_SCHEME_DISABLED = "EncryptionSchemeDisabled",

  // Configuration
  CIPHERNODE_REGISTRY_SET = "CiphernodeRegistrySet",
  MAX_DURATION_SET = "MaxDurationSet",
  ALLOWED_E3_PROGRAMS_PARAMS_SET = "AllowedE3ProgramsParamsSet",

  // Ownership
  OWNERSHIP_TRANSFERRED = "OwnershipTransferred",
  INITIALIZED = "Initialized",
}

export enum RegistryEventType {
  // Committee Management
  COMMITTEE_REQUESTED = "CommitteeRequested",
  COMMITTEE_PUBLISHED = "CommitteePublished",

  // Configuration
  ENCLAVE_SET = "EnclaveSet",

  // Ownership
  OWNERSHIP_TRANSFERRED = "OwnershipTransferred",
  INITIALIZED = "Initialized",
}

// Union type for all events
export type AllEventTypes = EnclaveEventType | RegistryEventType;

// Event data interfaces based on TypeChain types
export interface E3 {
  seed: bigint;
  threshold: readonly [number, number];
  requestBlock: bigint;
  startWindow: readonly [bigint, bigint];
  duration: bigint;
  expiration: bigint;
  encryptionSchemeId: string;
  e3Program: string;
  e3ProgramParams: string;
  inputValidator: string;
  decryptionVerifier: string;
  committeePublicKey: string;
  ciphertextOutput: string;
  plaintextOutput: string;
}

export interface E3RequestedData {
  e3Id: bigint;
  e3: E3;
  filter: string;
  e3Program: string;
}

export interface E3ActivatedData {
  e3Id: bigint;
  expiration: bigint;
  committeePublicKey: string;
}

export interface InputPublishedData {
  e3Id: bigint;
  data: string;
  inputHash: bigint;
  index: bigint;
}

export interface CiphertextOutputPublishedData {
  e3Id: bigint;
  ciphertextOutput: string;
}

export interface PlaintextOutputPublishedData {
  e3Id: bigint;
  plaintextOutput: string;
}

export interface CiphernodeAddedData {
  node: string;
  index: bigint;
  numNodes: bigint;
  size: bigint;
}

export interface CiphernodeRemovedData {
  node: string;
  index: bigint;
  numNodes: bigint;
  size: bigint;
}

export interface CommitteeRequestedData {
  e3Id: bigint;
  filter: string;
  threshold: [bigint, bigint];
}

export interface CommitteePublishedData {
  e3Id: bigint;
  publicKey: string;
}

// Event data mapping
export interface EnclaveEventData {
  [EnclaveEventType.E3_REQUESTED]: E3RequestedData;
  [EnclaveEventType.E3_ACTIVATED]: E3ActivatedData;
  [EnclaveEventType.INPUT_PUBLISHED]: InputPublishedData;
  [EnclaveEventType.CIPHERTEXT_OUTPUT_PUBLISHED]: CiphertextOutputPublishedData;
  [EnclaveEventType.PLAINTEXT_OUTPUT_PUBLISHED]: PlaintextOutputPublishedData;
  [EnclaveEventType.E3_PROGRAM_ENABLED]: { e3Program: string };
  [EnclaveEventType.E3_PROGRAM_DISABLED]: { e3Program: string };
  [EnclaveEventType.ENCRYPTION_SCHEME_ENABLED]: { encryptionSchemeId: string };
  [EnclaveEventType.ENCRYPTION_SCHEME_DISABLED]: { encryptionSchemeId: string };
  [EnclaveEventType.CIPHERNODE_REGISTRY_SET]: { ciphernodeRegistry: string };
  [EnclaveEventType.MAX_DURATION_SET]: { maxDuration: bigint };
  [EnclaveEventType.ALLOWED_E3_PROGRAMS_PARAMS_SET]: {
    e3ProgramParams: string[];
  };
  [EnclaveEventType.OWNERSHIP_TRANSFERRED]: {
    previousOwner: string;
    newOwner: string;
  };
  [EnclaveEventType.INITIALIZED]: { version: bigint };
}

export interface RegistryEventData {
  [RegistryEventType.COMMITTEE_REQUESTED]: CommitteeRequestedData;
  [RegistryEventType.COMMITTEE_PUBLISHED]: CommitteePublishedData;
  [RegistryEventType.ENCLAVE_SET]: { enclave: string };
  [RegistryEventType.OWNERSHIP_TRANSFERRED]: {
    previousOwner: string;
    newOwner: string;
  };
  [RegistryEventType.INITIALIZED]: { version: bigint };
}

// Generic event structure
export interface EnclaveEvent<T extends AllEventTypes> {
  type: T;
  data: T extends EnclaveEventType
  ? EnclaveEventData[T]
  : T extends RegistryEventType
  ? RegistryEventData[T]
  : unknown;
  log: Log;
  timestamp: Date;
  blockNumber: bigint;
  transactionHash: string;
}

export type EventCallback<T extends AllEventTypes = AllEventTypes> = (
  event: EnclaveEvent<T>,
) => void | Promise<void>;

export interface EventFilter<T = unknown> {
  address?: `0x${string}`;
  fromBlock?: bigint;
  toBlock?: bigint;
  args?: Partial<T>;
}

export interface SDKEventEmitter {
  on<T extends AllEventTypes>(eventType: T, callback: EventCallback<T>): void;
  off<T extends AllEventTypes>(eventType: T, callback: EventCallback<T>): void;
  emit<T extends AllEventTypes>(event: EnclaveEvent<T>): void;
}
