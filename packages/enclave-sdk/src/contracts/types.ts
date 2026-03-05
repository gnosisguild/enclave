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
