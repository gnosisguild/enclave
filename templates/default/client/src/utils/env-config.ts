// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

export const ENCLAVE_ADDRESS = import.meta.env.VITE_ENCLAVE_ADDRESS
export const E3_PROGRAM_ADDRESS = import.meta.env.VITE_E3_PROGRAM_ADDRESS
export const REGISTRY_ADDRESS = import.meta.env.VITE_REGISTRY_ADDRESS
export const BONDING_REGISTRY_ADDRESS = import.meta.env.VITE_BONDING_REGISTRY_ADDRESS
export const FEE_TOKEN_ADDRESS = import.meta.env.VITE_FEE_TOKEN_ADDRESS
export const RPC_URL = import.meta.env.VITE_RPC_URL || 'http://localhost:8545'

const requiredEnvVars = {
  VITE_ENCLAVE_ADDRESS: ENCLAVE_ADDRESS,
  VITE_E3_PROGRAM_ADDRESS: E3_PROGRAM_ADDRESS,
  VITE_REGISTRY_ADDRESS: REGISTRY_ADDRESS,
  VITE_BONDING_REGISTRY_ADDRESS: BONDING_REGISTRY_ADDRESS,
  VITE_FEE_TOKEN_ADDRESS: FEE_TOKEN_ADDRESS,
}

export const MISSING_ENV_VARS = Object.entries(requiredEnvVars)
  .filter(([, value]) => !value)
  .map(([key]) => key)

export const HAS_MISSING_ENV_VARS = MISSING_ENV_VARS.length > 0

/**
 * Validate environment variables and throw an error if any are missing
 */
export function validateEnvVars(): void {
  if (HAS_MISSING_ENV_VARS) {
    throw new Error(
      `Missing required environment variables: ${MISSING_ENV_VARS.join(', ')}\n` +
        'Please check your .env file and ensure all required variables are set.',
    )
  }
}

/**
 * Get validated contract addresses
 */
export function getContractAddresses() {
  validateEnvVars()
  return {
    enclave: ENCLAVE_ADDRESS as `0x${string}`,
    ciphernodeRegistry: REGISTRY_ADDRESS as `0x${string}`,
    bondingRegistry: BONDING_REGISTRY_ADDRESS as `0x${string}`,
    e3Program: E3_PROGRAM_ADDRESS as `0x${string}`,
    feeToken: FEE_TOKEN_ADDRESS as `0x${string}`,
  }
}
