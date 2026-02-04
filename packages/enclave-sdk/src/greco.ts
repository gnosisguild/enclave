// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { UltraHonkBackend, type ProofData } from '@aztec/bb.js'
import { type CompiledCircuit, Noir } from '@noir-lang/noir_js'

// Conversion to Noir types
export type Field = string

/**
 * Describes a polynomial to be used in a Noir circuit (Greco)
 */
export type Polynomial = {
  coefficients: Field[]
}

/**
 * Describes the inputs to Greco circuit
 */
export interface CircuitInputs {
  pk0is: string[][]
  pk1is: string[][]
  ct0is: string[][]
  ct1is: string[][]
  u: string[]
  e0: string[]
  e1: string[]
  e0is: string[][]
  e0_quotients: string[][]
  k1: string[]
  r1is: string[][]
  r2is: string[][]
  p1is: string[][]
  p2is: string[][]
  pk_commitment: string
}

/**
 * Generate a proof for a given circuit and circuit inputs
 * @dev Defaults to the UltraHonkBackend
 * @param circuitInputs - The circuit inputs
 * @param circuit - The circuit
 * @returns The proof
 */
export const generateProof = async (circuitInputs: CircuitInputs, circuit: CompiledCircuit): Promise<ProofData> => {
  const noir = new Noir(circuit)

  const backend = new UltraHonkBackend(circuit.bytecode, { threads: 4 })

  const { witness } = await noir.execute(circuitInputs as any)

  return await backend.generateProof(witness, { keccakZK: true })
}
