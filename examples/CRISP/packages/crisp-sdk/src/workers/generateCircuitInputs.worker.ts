// SPDX-License-Identifier: LGPL-3.0-only
//
// Runs generateCircuitInputs in a worker to avoid blocking the main thread
// during CPU-heavy zk-inputs WASM (BFV encryption).

import type { ProofInputs } from '../types'
import { generateCircuitInputsImpl } from '../circuitInputs'

self.onmessage = async (e: MessageEvent<ProofInputs>) => {
  try {
    const result = await generateCircuitInputsImpl(e.data)
    self.postMessage({ type: 'result' as const, ...result })
  } catch (err) {
    const error = err instanceof Error ? err.message : String(err)
    const stack = err instanceof Error ? err.stack : undefined
    self.postMessage({ type: 'error' as const, error, stack })
  }
}
