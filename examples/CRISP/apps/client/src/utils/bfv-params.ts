import { encodePacked, encodeAbiParameters } from 'viem'

// BFV parameter set matching the Rust SET_2048_1032193_1 configuration
export const BFV_PARAMS_SET = {
    degree: 2048,
    plaintext_modulus: 1032193,
    moduli: [0x3FFFFFFF000001n] // BigInt for the modulus
} as const

// Compute provider parameters structure
export interface ComputeProviderParams {
    name: string
    parallel: boolean
    batch_size: number
}

// Default compute provider configuration
export const DEFAULT_COMPUTE_PROVIDER_PARAMS: ComputeProviderParams = {
    name: "risc0",
    parallel: false,
    batch_size: 2
}

// Default E3 configuration
export const DEFAULT_E3_CONFIG = {
    threshold_min: 2,
    threshold_max: 3,
    window_size: 300, // 5 minutes in seconds
    duration: 1800, // 30 minutes in seconds
    payment_amount: "0" // 0 ETH in wei
} as const

/**
 * Encode BFV parameters to bytes for the smart contract
 * This matches the Rust implementation: encode_bfv_params(&build_bfv_params_arc(degree, plaintext_modulus, &moduli))
 */
export function encodeBfvParams(
    degree: number = BFV_PARAMS_SET.degree,
    plaintext_modulus: number = BFV_PARAMS_SET.plaintext_modulus,
    moduli: readonly bigint[] = BFV_PARAMS_SET.moduli
): `0x${string}` {
    // Encode as (uint256, uint256, uint256[]) - matching the Solidity struct
    return encodeAbiParameters(
        [
            { name: 'degree', type: 'uint256' },
            { name: 'plaintext_modulus', type: 'uint256' },
            { name: 'moduli', type: 'uint256[]' }
        ],
        [BigInt(degree), BigInt(plaintext_modulus), [...moduli]]
    )
}

/**
 * Encode compute provider parameters to bytes
 * This simulates the Rust bincode::serialize(&compute_provider_params)
 */
export function encodeComputeProviderParams(params: ComputeProviderParams): `0x${string}` {
    // For simplicity, we'll encode as a JSON string and then as bytes
    // In a real implementation, this might need to match the exact Rust bincode format
    const jsonString = JSON.stringify(params)
    const encoder = new TextEncoder()
    const bytes = encoder.encode(jsonString)

    // Convert to hex string
    return `0x${Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('')}`
}

/**
 * Get the current timestamp in seconds
 */
export function getCurrentTimestamp(): number {
    return Math.floor(Date.now() / 1000)
}

/**
 * Calculate start window for E3 request
 */
export function calculateStartWindow(windowSize: number = DEFAULT_E3_CONFIG.window_size): [bigint, bigint] {
    const now = getCurrentTimestamp()
    return [BigInt(now), BigInt(now + windowSize)]
} 