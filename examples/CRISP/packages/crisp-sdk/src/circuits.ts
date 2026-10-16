// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import type { CompiledCircuit } from '@noir-lang/noir_js'

/** BFV parameter sets the circuits can be compiled against. */
export type CircuitPreset = 'insecure-64' | 'secure-8192'

/**
 * The circuits whose ABI is shaped by the BFV degree, and which therefore exist once per preset.
 *
 * The fold circuits — `crisp_fold` and `crisp_onchain_fold` — are deliberately absent. Their
 * parameters are proof and verification-key shaped (410/115 fields), not polynomial shaped, so one
 * compiled artifact serves both presets; they assert `chain_key_hash` against the insecure *or* the
 * secure constant for exactly that reason. They ship in the main entry point, which is why
 * `verifyProof` works without a preset loaded.
 */
export type CircuitBundle = {
  readonly preset: CircuitPreset
  readonly crisp: CompiledCircuit
  readonly crispOnchain: CompiledCircuit
  /** Proves both legs of the ballot's BFV encryption in one circuit. */
  readonly userDataEncryption: CompiledCircuit
  /**
   * The `noir-recursive-no-zk` verification keys of the circuits above, written with their bytecode.
   * bb.js's `generateRecursiveProofArtifacts` would rebuild each circuit to recompute them on every
   * ballot.
   */
  readonly vks: {
    readonly crisp: RecursiveVk
    readonly crispOnchain: RecursiveVk
    readonly userDataEncryption: RecursiveVk
  }
}

/** A verification key in the shape `generateRecursiveProofArtifacts` returns it. */
export type RecursiveVk = { readonly vkAsFields: string[]; readonly vkHash: string }

let registered: CircuitBundle | null = null

/**
 * Install the preset-bound circuits used by `generateProof`.
 *
 * The bundle is not bundled into the main entry point, because the secure-8192 artifacts are more
 * than an order of magnitude larger than the insecure-64 ones. Load the preset selected by the
 * round from its subpath and register it before proving:
 *
 * ```ts
 * import { setCircuits } from '@crisp-e3/sdk'
 * import { loadCircuits } from '@crisp-e3/sdk/insecure-64'
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
      'No circuit preset registered. Import `loadCircuits` from "@crisp-e3/sdk/insecure-64" or ' +
        '"@crisp-e3/sdk/secure-8192" and pass the result to `setCircuits()` before proving.',
    )
  }

  return registered
}
