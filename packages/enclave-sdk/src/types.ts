// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import type { Log, PublicClient, WalletClient } from "viem";
import type { ProofData } from "@aztec/bb.js";
import type {
  CiphernodeRegistryOwnable,
  Enclave,
  MockCiphernodeRegistry,
  MockUSDC,
  EnclaveToken,
} from "@enclave-e3/contracts/types";

import type { CircuitInputs } from "./greco";

/**
 * SDK configuration
 */
export interface SDKConfig {
  /**
   * The public client to use to interact with the blockchain
   */
  publicClient: PublicClient;

  /**
   * The wallet client to use to send/sign transactions
   */
  walletClient?: WalletClient;

  /**
   * The Enclave contracts
   */
  contracts: {
    /**
     * The Enclave contract address
     */
    enclave: `0x${string}`;

    /**
     * The CiphernodeRegistry contract address
     */
    ciphernodeRegistry: `0x${string}`;

    /**
     * The FeeToken contract address
     */
    feeToken: `0x${string}`;
  };

  /**
   * The chain ID to which the contracts are deployed
   */
  chainId?: number;

  /**
   * The protocol to use for the Enclave requests
   */
  protocol: FheProtocol;

  /**
   * The protocol parameters to use for the Enclave requests
   */
  protocolParams?: ProtocolParams;
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
  feeToken: EnclaveToken | MockUSDC;
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
  COMMITTEE_FINALIZED = "CommitteeFinalized",

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
  seed: bigint;
  threshold: [bigint, bigint];
  requestBlock: bigint;
  submissionDeadline: bigint;
}

export interface CommitteePublishedData {
  e3Id: bigint;
  publicKey: string;
}

export interface CommitteeFinalizedData {
  e3Id: bigint;
  nodes: string[];
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
  [RegistryEventType.COMMITTEE_FINALIZED]: CommitteeFinalizedData;
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
  event: EnclaveEvent<T>
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

/**
 * Result of verifiable encryption using BFV
 */
export interface VerifiableEncryptionResult {
  /**
   * The encrypted data
   */
  encryptedData: Uint8Array;
  /**
   * The proof generated by Greco
   */
  proof: ProofData;
}

/**
 * The protocol to use for the Enclave requests
 */
export enum FheProtocol {
  /**
   * The BFV protocol
   */
  BFV = "BFV",
  /**
   * The TrBFV protocol
   */
  TRBFV = "TRBFV",
}

/**
 * Protocol parameters for an Enclave program request
 * Example for BFV
 *   2048,               // degree
 *   1032193,            // plaintext_modulus
 *   0x3FFFFFFF000001,   // moduli
 */
export interface ProtocolParams {
  /**
   * The degree of the polynomial
   */
  degree: number;
  /**
   * The plaintext modulus
   */
  plaintextModulus: bigint;
  /**
   * The moduli
   */
  moduli: bigint[];
}

/**
 * Parameters for the BFV protocol
 */
export const BfvProtocolParams = {
  /**
   * Recommended parameters for BFV protocol
   * - Degree: 2048
   * - Plaintext modulus: 1032193
   * - Moduli:0x3FFFFFFF000001
   */
  BFV_NORMAL: {
    degree: 2048,
    plaintextModulus: 1032193n,
    moduli: [0x3fffffff000001n],
  } as const satisfies ProtocolParams,

  /**
   * Recommended parameters for TrBFV protocol
   * - Degree: 8192
   * - Plaintext modulus: 1000
   * - Moduli: [0x00800000022a0001, 0x00800000021a0001, 0x0080000002120001, 0x0080000001f60001]
   */
  BFV_THRESHOLD: {
    degree: 8192,
    plaintextModulus: 1000n,
    moduli: [
      0x00800000022a0001n,
      0x00800000021a0001n,
      0x0080000002120001n,
      0x0080000001f60001n,
    ],
  } as const satisfies ProtocolParams,
};

/**
 * The result of encrypting a value and generating a proof
 */
export interface EncryptedValueAndPublicInputs {
  /**
   * The encrypted data
   */
  encryptedData: Uint8Array;

  /**
   * The public inputs for the proof
   */
  publicInputs: CircuitInputs;
}
