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

export interface E3 {
  seed: bigint
  threshold: readonly [number, number]
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

export interface RequestParams {
  gasLimit?: bigint
}

export interface E3RequestParams extends RequestParams {
  threshold: readonly [number, number]
  inputWindow: readonly [bigint, bigint]
  e3Program: `0x${string}`
  e3ProgramParams: `0x${string}`
  computeProviderParams: `0x${string}`
  customParams?: `0x${string}`
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
