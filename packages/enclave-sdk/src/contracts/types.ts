// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

export interface ContractAddresses {
  enclave: `0x${string}`
  ciphernodeRegistry: `0x${string}`
  feeToken: `0x${string}`
}

export enum CommitteeSize {
  Micro = 0,
  Small = 1,
  Medium = 2,
  Large = 3,
}

export enum ParamSet {
  Insecure512 = 0,
  Secure8192 = 1,
}

export interface E3 {
  seed: bigint
  committeeSize: number
  requestBlock: bigint
  inputWindow: readonly [bigint, bigint]
  encryptionSchemeId: string
  e3Program: string
  paramSet: number
  decryptionVerifier: string
  committeePublicKey: string
  ciphertextOutput: string
  plaintextOutput: string
}

export interface RequestParams {
  gasLimit?: bigint
}

export interface E3RequestParams extends RequestParams {
  committeeSize: number
  inputWindow: readonly [bigint, bigint]
  e3Program: `0x${string}`
  paramSet: number
  computeProviderParams: `0x${string}`
  customParams?: `0x${string}`
  /** When true, ciphernodes generate wrapper/fold proofs for DKG proof aggregation.
   *  When false, proof aggregation is skipped for faster computation. Defaults to true. */
  proofAggregationEnabled?: boolean
}

export enum E3Stage {
  None,
  Requested,
  CommitteeFinalized,
  KeyPublished,
  CiphertextReady,
  Complete,
  Failed,
}

export enum FailureReason {
  None,
  CommitteeFormationTimeout,
  InsufficientCommitteeMembers,
  DKGTimeout,
  DKGInvalidShares,
  NoInputsReceived,
  ComputeTimeout,
  ComputeProviderExpired,
  ComputeProviderFailed,
  RequesterCancelled,
  DecryptionTimeout,
  DecryptionInvalidShares,
  VerificationFailed,
}
