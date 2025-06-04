import contractABI from '@/abis/enclave.abi.json'

// Environment variables with validation
export const ENCLAVE_ADDRESS = import.meta.env.VITE_ENCLAVE_ADDRESS
export const E3_PROGRAM_ADDRESS = import.meta.env.VITE_E3_PROGRAM_ADDRESS
export const REGISTRY_ADDRESS = import.meta.env.VITE_REGISTRY_ADDRESS
export const FILTER_REGISTRY_ADDRESS = import.meta.env.VITE_FILTER_REGISTRY_ADDRESS

// Validate required environment variables
const requiredEnvVars = {
    VITE_ENCLAVE_ADDRESS: ENCLAVE_ADDRESS,
    VITE_E3_PROGRAM_ADDRESS: E3_PROGRAM_ADDRESS,
    VITE_REGISTRY_ADDRESS: REGISTRY_ADDRESS,
    VITE_FILTER_REGISTRY_ADDRESS: FILTER_REGISTRY_ADDRESS,
}

export const MISSING_ENV_VARS = Object.entries(requiredEnvVars)
    .filter(([, value]) => !value)
    .map(([key]) => key)

export const HAS_MISSING_ENV_VARS = MISSING_ENV_VARS.length > 0

// Contract ABIs
export const ENCLAVE_ABI = contractABI.abi
export const REGISTRY_ABI = [
    {
        type: 'event',
        name: 'CommitteePublished',
        inputs: [
            {
                name: 'e3Id',
                type: 'uint256',
                indexed: true,
            },
            {
                name: 'publicKey',
                type: 'bytes',
                indexed: false,
            },
        ],
    },
] as const 