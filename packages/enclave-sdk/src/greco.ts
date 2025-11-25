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
 * Convert a string array to a polynomial
 * @param stringArray - The string array
 * @returns The polynomial
 */
export const convertToPolynomial = (stringArray: string[]): Polynomial => {
  return {
    coefficients: stringArray,
  }
}

/**
 * Convert an array of string arrays to an array of polynomials
 * @param stringArrays - The array of string arrays
 * @returns The array of polynomials
 */
export const convertToPolynomialArray = (stringArrays: string[][]): Polynomial[] => {
  return stringArrays.map(convertToPolynomial)
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

  const pk0is_poly = convertToPolynomialArray(circuitInputs.pk0is)
  const pk1is_poly = convertToPolynomialArray(circuitInputs.pk1is)
  const ct0is_poly = convertToPolynomialArray(circuitInputs.ct0is)
  const ct1is_poly = convertToPolynomialArray(circuitInputs.ct1is)
  const u_poly = convertToPolynomial(circuitInputs.u)
  const e0_poly = convertToPolynomial(circuitInputs.e0)
  const e1_poly = convertToPolynomial(circuitInputs.e1)
  const e0is_poly = convertToPolynomialArray(circuitInputs.e0is)
  const k1_poly = convertToPolynomial(circuitInputs.k1)
  const r1is_poly = convertToPolynomialArray(circuitInputs.r1is)
  const r2is_poly = convertToPolynomialArray(circuitInputs.r2is)
  const p1is_poly = convertToPolynomialArray(circuitInputs.p1is)
  const p2is_poly = convertToPolynomialArray(circuitInputs.p2is)

  const { witness } = await noir.execute({
    params: defaultParams as any,
    pk0is: pk0is_poly,
    pk1is: pk1is_poly,
    ct0is: ct0is_poly,
    ct1is: ct1is_poly,
    u: u_poly,
    e0: e0_poly,
    e1: e1_poly,
    e0is: e0is_poly,
    k1: k1_poly,
    r1is: r1is_poly,
    r2is: r2is_poly,
    p1is: p1is_poly,
    p2is: p2is_poly,
  })

  return await backend.generateProof(witness, { keccakZK: true })
}
