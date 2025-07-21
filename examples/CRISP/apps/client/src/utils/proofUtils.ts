// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { UltraHonkBackend, ProofData } from '@aztec/bb.js';
import { Noir } from '@noir-lang/noir_js';
import crisp_circuit from 'libs/noir/crisp_circuit.json';

export type Field = string;

export type Polynomial = {
    coefficients: Field[];
};

export interface CircuitInputs {
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

export const convertToPolynomial = (stringArray: string[]): Polynomial => {
    return {
        coefficients: stringArray,
    };
};

export const convertToPolynomialArray = (
    stringArrays: string[][]
): Polynomial[] => {
    return stringArrays.map(convertToPolynomial);
};

export const generateProof = async (circuitInputs: CircuitInputs): Promise<ProofData> => {
    const noir = new Noir(crisp_circuit as any);
    const backend = new UltraHonkBackend(crisp_circuit.bytecode, { threads: 4 });

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