// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

export const ENCLAVE_ADDRESS = import.meta.env.VITE_ENCLAVE_ADDRESS
export const E3_PROGRAM_ADDRESS = import.meta.env.VITE_E3_PROGRAM_ADDRESS
export const REGISTRY_ADDRESS = import.meta.env.VITE_REGISTRY_ADDRESS
export const FILTER_REGISTRY_ADDRESS = import.meta.env.VITE_FILTER_REGISTRY_ADDRESS
export const RPC_URL = import.meta.env.VITE_RPC_URL || 'http://localhost:8545'

// Get the missing environment variables.
// This is used to check if the environment variables are set.
export const MISSING_ENV_VARS = Object.entries({
  VITE_ENCLAVE_ADDRESS: ENCLAVE_ADDRESS,
  VITE_E3_PROGRAM_ADDRESS: E3_PROGRAM_ADDRESS,
  VITE_REGISTRY_ADDRESS: REGISTRY_ADDRESS,
  VITE_FILTER_REGISTRY_ADDRESS: FILTER_REGISTRY_ADDRESS,
})
  .filter(([, value]) => !value)
  .map(([key]) => key)

/**
 * Get validated contract addresses.
 */
export function getContractAddresses() {
  return {
    enclave: ENCLAVE_ADDRESS as `0x${string}`,
    ciphernodeRegistry: REGISTRY_ADDRESS as `0x${string}`,
    filterRegistry: FILTER_REGISTRY_ADDRESS as `0x${string}`,
    e3Program: E3_PROGRAM_ADDRESS as `0x${string}`,
  }
}
