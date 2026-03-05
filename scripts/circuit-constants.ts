// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/**
 * Shared constants for circuit groups used across build and verifier generation scripts.
 */

export const CIRCUIT_GROUPS = {
  DKG: 'dkg',
  THRESHOLD: 'threshold',
  AGGREGATION: 'recursive_aggregation',
} as const

export type CircuitGroup = (typeof CIRCUIT_GROUPS)[keyof typeof CIRCUIT_GROUPS]

export const ALL_GROUPS: CircuitGroup[] = [CIRCUIT_GROUPS.DKG, CIRCUIT_GROUPS.THRESHOLD, CIRCUIT_GROUPS.AGGREGATION]

/**
 * Circuit variants determine the hash oracle used for VK generation and proving.
 *
 * - `default`: Uses poseidon/noir-recursive-no-zk — wrapper & fold proofs (efficient, no ZK blinding).
 * - `recursive`: Uses poseidon/noir-recursive — inner/base proofs (ZK blinding preserved).
 * - `evm`: Uses keccak — for on-chain EVM-verifiable proofs.
 */
export const CIRCUIT_VARIANTS = {
  DEFAULT: 'default',
  RECURSIVE: 'recursive',
  EVM: 'evm',
} as const

export type CircuitVariant = (typeof CIRCUIT_VARIANTS)[keyof typeof CIRCUIT_VARIANTS]

export const ALL_VARIANTS: CircuitVariant[] = [CIRCUIT_VARIANTS.DEFAULT, CIRCUIT_VARIANTS.RECURSIVE, CIRCUIT_VARIANTS.EVM]
