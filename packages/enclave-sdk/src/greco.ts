// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { UltraHonkBackend, type ProofData } from '@aztec/bb.js';
import { type CompiledCircuit, Noir } from '@noir-lang/noir_js';

// Conversion to Noir types
export type Field = string;

/**
 * Describes a polynomial to be used in a Noir circuit (Greco)
 */
export type Polynomial = {
    coefficients: Field[];
};

/**
 * Describes the inputs to Greco circuit
 */
export interface CircuitInputs {
    params: Params,
    pk0is: string[][];
    pk1is: string[][];
    ct0is: string[][];
    ct1is: string[][];
    u: string[];
    e0: string[];
    e1: string[];
    k1: string[];
    r1is: string[][];
    r2is: string[][];
    p1is: string[][];
    p2is: string[][];
}

/**
 * BfvPkEncryption params for Greco
pub struct Params<let N: u32, let L: u32> {
    q_mod_t: Field,
    pk_bounds: [u64; L],
    e_bound: u64,
    u_bound: u64,
    r1_low_bounds: [i64; L],
    r1_up_bounds: [u64; L],
    r2_bounds: [u64; L],
    p1_bounds: [u64; L],
    p2_bounds: [u64; L],
    k1_low_bound: i64,
    k1_up_bound: u64,
    qis: [Field; L],
    k0is: [Field; L],
    size: u32,
    tag: Field,
}
 */
export interface Params {
    q_mod_t: Field,
    pk_bounds: Field[],
    e_bound: Field,
    u_bound: Field,
    r1_low_bounds: Field[],
    r1_up_bounds: Field[],
    r2_bounds: Field[],
    p1_bounds: Field[],
    p2_bounds: Field[],
    k1_low_bound: Field,
    k1_up_bound: Field,
    qis: Field[],
    k0is: Field[],
    size: Field,
    tag: Field,
}

/**
 * Default greco params for BFV pk encryption
 * Generated using https://github.com/gnosisguild/greco/tree/main/crates/generator
 */
export const defaultParams: Params = {
    q_mod_t: "21888242871839275222246405745257275088548364400416034343698204186575808249358",
    pk_bounds: ["9007199246352384"],
    e_bound: "19",
    u_bound: "19",
    r1_low_bounds: ["-481795"],
    r1_up_bounds: ["481795"],
    r2_bounds: ["9007199246352384"],
    p1_bounds: ["19456"],
    p2_bounds: ["9007199246352384"],
    k1_low_bound: "-516096",
    k1_up_bound: "516096",
    qis: ["18014398492704769"],
    k0is: ["16137970277882884"],
    size: "28668",
    tag: "5380324561195082",
}

/**
 * Convert a string array to a polynomial
 * @param stringArray - The string array
 * @returns The polynomial
 */
export const convertToPolynomial = (stringArray: string[]): Polynomial => {
    return {
        coefficients: stringArray,
    };
};

/**
 * Convert an array of string arrays to an array of polynomials
 * @param stringArrays - The array of string arrays
 * @returns The array of polynomials
 */
export const convertToPolynomialArray = (
    stringArrays: string[][]
): Polynomial[] => {
    return stringArrays.map(convertToPolynomial);
};

/**
 * Generate a proof for a given circuit and circuit inputs
 * @dev Defaults to the UltraHonkBackend
 * @param circuitInputs - The circuit inputs
 * @param circuit - The circuit
 * @returns The proof
 */
export const generateProof = async (circuitInputs: CircuitInputs, circuit: CompiledCircuit): Promise<ProofData> => {
    const noir = new Noir(circuit);

    const backend = new UltraHonkBackend(circuit.bytecode, { threads: 4 });

    const pk0is_poly = convertToPolynomialArray(circuitInputs.pk0is);
    const pk1is_poly = convertToPolynomialArray(circuitInputs.pk1is);
    const ct0is_poly = convertToPolynomialArray(circuitInputs.ct0is);
    const ct1is_poly = convertToPolynomialArray(circuitInputs.ct1is);
    const u_poly = convertToPolynomial(circuitInputs.u);
    const e0_poly = convertToPolynomial(circuitInputs.e0);
    const e1_poly = convertToPolynomial(circuitInputs.e1);
    const k1_poly = convertToPolynomial(circuitInputs.k1);
    const r1is_poly = convertToPolynomialArray(circuitInputs.r1is);
    const r2is_poly = convertToPolynomialArray(circuitInputs.r2is);
    const p1is_poly = convertToPolynomialArray(circuitInputs.p1is);
    const p2is_poly = convertToPolynomialArray(circuitInputs.p2is);

    const { witness } = await noir.execute({
        params: {
            q_mod_t: circuitInputs.params.q_mod_t,
            pk_bounds: circuitInputs.params.pk_bounds,
            e_bound: circuitInputs.params.e_bound,
            u_bound: circuitInputs.params.u_bound,
            r1_low_bounds: circuitInputs.params.r1_low_bounds,
            r1_up_bounds: circuitInputs.params.r1_up_bounds,
            r2_bounds: circuitInputs.params.r2_bounds,
            p1_bounds: circuitInputs.params.p1_bounds,
            p2_bounds: circuitInputs.params.p2_bounds,
            k1_low_bound: circuitInputs.params.k1_low_bound,
            k1_up_bound: circuitInputs.params.k1_up_bound,
            qis: circuitInputs.params.qis,
            k0is: circuitInputs.params.k0is,
            size: circuitInputs.params.size,
            tag: circuitInputs.params.tag,
        },
        pk0is: pk0is_poly,
        pk1is: pk1is_poly,
        ct0is: ct0is_poly,
        ct1is: ct1is_poly,
        u: u_poly,
        e0: e0_poly,
        e1: e1_poly,
        k1: k1_poly,
        r1is: r1is_poly,
        r2is: r2is_poly,
        p1is: p1is_poly,
        p2is: p2is_poly,
    });

    return await backend.generateProof(witness, { keccak: true });
}; 
