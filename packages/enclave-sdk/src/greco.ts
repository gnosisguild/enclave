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
  k1: string[]
  r1is: string[][]
  r2is: string[][]
  p1is: string[][]
  p2is: string[][]
}

/**
 * BfvPkEncryption params for Greco
pub struct Params<let N: u32, let L: u32> {
    crypto: CryptographicParams<L>,
    bounds: BoundParams<L>,
}
 */
export interface BoundParams {
  pk_bounds: Field[]
  e0_bound: Field
  e1_bound: Field
  u_bound: Field
  r1_low_bounds: Field[]
  r1_up_bounds: Field[]
  r2_bounds: Field[]
  p1_bounds: Field[]
  p2_bounds: Field[]
  k1_low_bound: Field
  k1_up_bound: Field
}

export interface CryptographicParams {
  q_mod_t: Field
  qis: Field[]
  k0is: Field[]
}

export interface Params {
  bounds: BoundParams
  crypto: CryptographicParams
}

/**
 * Default greco params for BFV pk encryption.
 */
export const defaultParams: Params = {
  bounds: {
    pk_bounds: ['34359701504', '34359615488'],
    e0_bound: '20',
    e1_bound: '20',
    u_bound: '1',
    r1_low_bounds: ['261', '258'],
    r1_up_bounds: ['260', '258'],
    r2_bounds: ['34359701504', '34359615488'],
    p1_bounds: ['256', '256'],
    p2_bounds: ['34359701504', '34359615488'],
    k1_low_bound: '5',
    k1_up_bound: '4',
  },
  crypto: {
    q_mod_t: '3',
    qis: ['68719403009', '68719230977'],
    k0is: ['61847462708', '20615769293'],
  },
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

  const { witness } = await noir.execute({
    params: defaultParams as any,
    pk0is: circuitInputs.pk0is,
    pk1is: circuitInputs.pk1is,
    ct0is: circuitInputs.ct0is,
    ct1is: circuitInputs.ct1is,
    u: circuitInputs.u,
    e0: circuitInputs.e0,
    e1: circuitInputs.e1,
    e0is: circuitInputs.e0is,
    k1: circuitInputs.k1,
    r1is: circuitInputs.r1is,
    r2is: circuitInputs.r2is,
    p1is: circuitInputs.p1is,
    p2is: circuitInputs.p2is,
  })

  return await backend.generateProof(witness, { keccakZK: true })
}
