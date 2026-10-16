// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// The insecure-512 (N=512, L=2) preset-bound circuits, published as a separate entry point.
//
// Kept out of the main entry so a consumer's bundler pulls one preset rather than both. The
// artifacts are staged by scripts/stage-preset-artifacts.mjs; see src/circuits.ts for why the
// aggregation circuits are not among them.

import type { CircuitBundle, CircuitPreset } from '../circuits'
import type { CompiledCircuit } from '@noir-lang/noir_js'

import crisp from '../../../../circuits/dist/insecure-512/crisp.json'
import crispOnchain from '../../../../circuits/dist/insecure-512/crisp_onchain.json'
import userDataEncryption from '../../../../circuits/dist/insecure-512/user_data_encryption.json'
import crispVk from '../../../../circuits/dist/insecure-512/crisp.vk.json'
import crispOnchainVk from '../../../../circuits/dist/insecure-512/crisp_onchain.vk.json'
import userDataEncryptionVk from '../../../../circuits/dist/insecure-512/user_data_encryption.vk.json'

export const preset: CircuitPreset = 'insecure-512'

/**
 * The insecure-512 (N=512, L=2) circuits, ready for `setCircuits()`.
 *
 * Asynchronous because the artifacts are inlined today but need not stay that way: the secure set
 * is large enough that a consumer may want it fetched on demand, and that change belongs inside
 * this function rather than in every caller.
 */
export const loadCircuits = async (): Promise<CircuitBundle> => ({
  preset,
  crisp: crisp as CompiledCircuit,
  crispOnchain: crispOnchain as CompiledCircuit,
  userDataEncryption: userDataEncryption as CompiledCircuit,
  vks: {
    crisp: crispVk,
    crispOnchain: crispOnchainVk,
    userDataEncryption: userDataEncryptionVk,
  },
})
