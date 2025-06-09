import { UltraHonkBackend, ProofData } from '@aztec/bb.js'
import { Noir } from '@noir-lang/noir_js'
import crisp_circuit from 'libs/noir/crisp_circuit.json'

export type Field = string

export type Polynomial = {
  coefficients: Field[]
}

export interface CircuitInputs {
  pk0is: string[][]
  pk1is: string[][]
  ct0is: string[][]
  ct1is: string[][]
  u: string[]
  e0: string[]
  e1: string[]
  k1: string[]
  r1is: string[][]
  r2is: string[][]
  p1is: string[][]
  p2is: string[][]
}

// Helper to validate that a string represents a valid field element
const isValidFieldElement = (value: string): boolean => {
  try {
    // Check if it's a valid number and within the Noir field size
    const num = BigInt(value)
    const NOIR_FIELD_SIZE = BigInt('21888242871839275222246405745257275088548364400416034343698204186575808495617')
    return num >= 0n && num < NOIR_FIELD_SIZE
  } catch {
    return false
  }
}

export const convertToPolynomial = (stringArray: string[]): Polynomial => {
  // Validate each coefficient
  stringArray.forEach((value, index) => {
    if (!isValidFieldElement(value)) {
      throw new Error(`Invalid field element at index ${index}: ${value}`)
    }
  })

  return {
    coefficients: stringArray,
  }
}

export const convertToPolynomialArray = (stringArrays: string[][]): Polynomial[] => {
  return stringArrays.map((arr, i) => {
    try {
      return convertToPolynomial(arr)
    } catch (e: any) {
      throw new Error(`Error in array ${i}: ${e.message}`)
    }
  })
}

export const generateProof = async (circuitInputs: CircuitInputs): Promise<ProofData> => {
  console.log('Starting proof generation with inputs:', JSON.stringify(circuitInputs, null, 2))
  console.log('circuitInputs', circuitInputs)
  try {
    const noir = new Noir(crisp_circuit as any)
    const backend = new UltraHonkBackend(crisp_circuit.bytecode, {
      threads: 4,
    })

    // Convert and validate all inputs
    console.log('Converting and validating inputs...')

    const inputs = {
      pk0is: convertToPolynomialArray(circuitInputs.pk0is),
      pk1is: convertToPolynomialArray(circuitInputs.pk1is),
      ct0is: convertToPolynomialArray(circuitInputs.ct0is),
      ct1is: convertToPolynomialArray(circuitInputs.ct1is),
      u: convertToPolynomial(circuitInputs.u),
      e0: convertToPolynomial(circuitInputs.e0),
      e1: convertToPolynomial(circuitInputs.e1),
      k1: convertToPolynomial(circuitInputs.k1),
      r1is: convertToPolynomialArray(circuitInputs.r1is),
      r2is: convertToPolynomialArray(circuitInputs.r2is),
      p1is: convertToPolynomialArray(circuitInputs.p1is),
      p2is: convertToPolynomialArray(circuitInputs.p2is),
    }

    console.log('All inputs converted and validated successfully')
    console.log('Executing Noir program...')

    const { witness } = await noir.execute(inputs)

    console.log('Witness generated successfully')
    console.log('Generating final proof...')

    return await backend.generateProof(witness, { keccak: true })
  } catch (error) {
    console.error('Error during proof generation:', error)
    throw error
  }
}
