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
