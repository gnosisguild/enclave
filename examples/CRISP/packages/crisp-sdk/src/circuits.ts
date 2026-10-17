// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import type { CompiledCircuit } from '@noir-lang/noir_js'

/** BFV parameter sets the circuits can be compiled against. */
export type CircuitPreset = 'insecure' | 'secure-8192' | 'secure-16384'

/**
 * The circuits whose bytecode depends on the BFV preset.
 *
 * The aggregation circuits — `crisp_fold`, `crisp_onchain_fold`, and `user_data_encryption_chunked` — are
 * deliberately absent. Their parameters are proof and verification-key shaped (410/115 fields), not
 * polynomial shaped, so one compiled artifact serves all presets; the fold circuits assert
 * `chain_key_hash` against the insecure *or* the secure constant for exactly that reason. They ship
 * in the main entry point, which is why `verifyProof` works without a preset loaded.
 */
export type PresetCircuitName =
  | 'crisp'
  | 'crispOnchain'
  | 'ct0ChunkMain'
  | 'ct0PkCtCommit'
  | 'ct0EvalChunkIdentity'
  | 'userDataEncryptionCt0'
  | 'ct1ChunkMain'
  | 'ct1PkCtCommit'
  | 'ct1EvalChunkIdentity'
  | 'userDataEncryptionCt1'

export type CircuitBundle = { readonly preset: CircuitPreset } & { readonly [K in PresetCircuitName]: CompiledCircuit } & {
  /**
   * Each circuit's recursive verification key, written with its bytecode. bb.js rebuilds a whole
   * circuit to return its key, so shipping them saves one circuit build per proof in a ballot.
   */
  readonly vks: { readonly [K in PresetCircuitName]: RecursiveVk }
}

/** A recursive verification key, as bb.js `generateRecursiveProofArtifacts` returns it. */
export type RecursiveVk = { readonly vkAsFields: string[]; readonly vkHash: string }

let registered: CircuitBundle | null = null

/**
 * Install the preset-bound circuits used by `generateProof`.
 *
 * The bundle is not bundled into the main entry point, because the secure artifacts are more
 * than an order of magnitude larger than the insecure ones. Load the preset selected by the
 * round from its subpath and register it before proving:
 *
 * ```ts
 * import { setCircuits } from '@crisp-e3/sdk'
 * import { loadCircuits } from '@crisp-e3/sdk/insecure'
 *
 * setCircuits(await loadCircuits())
 * ```
 */
export const setCircuits = (bundle: CircuitBundle): void => {
  registered = bundle
}

/** The registered bundle, or `null` when none has been installed yet. */
export const getRegisteredCircuits = (): CircuitBundle | null => registered

/** The preset currently installed, or `null` when none has been installed yet. */
export const registeredPreset = (): CircuitPreset | null => registered?.preset ?? null

/**
 * The registered bundle, throwing a directed error when nothing has been installed.
 *
 * Proving cannot fall back to a default preset: a ballot proved against the wrong parameters fails
 * on chain rather than locally, so guessing here would move the failure somewhere much harder to
 * read.
 */
export const requireCircuits = (): CircuitBundle => {
  if (!registered) {
    throw new Error(
      'No circuit preset registered. Import `loadCircuits` from "@crisp-e3/sdk/insecure", ' +
        '"@crisp-e3/sdk/secure-8192", or "@crisp-e3/sdk/secure-16384" and pass the result ' +
        'to `setCircuits()` before proving.',
    )
  }

  return registered
}
