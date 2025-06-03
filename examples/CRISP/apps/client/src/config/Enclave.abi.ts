import { Abi } from 'viem'
import EnclaveArtifact from './Enclave.json'

// Environment variables
export const ENCLAVE_ADDRESS = import.meta.env.VITE_ENCLAVE_ADDRESS
export const E3_PROGRAM_ADDRESS = import.meta.env.VITE_E3_PROGRAM_ADDRESS
export const REGISTRY_ADDRESS = import.meta.env.VITE_REGISTRY_ADDRESS
export const FILTER_REGISTRY_ADDRESS = import.meta.env.VITE_FILTER_REGISTRY_ADDRESS

// Check for missing environment variables
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

// Use the correct ABI from the contract artifact
export const ENCLAVE_ABI = EnclaveArtifact.abi as Abi

export const E3_PROGRAM_ABI = [
    {
        "inputs": [
            {
                "internalType": "uint256",
                "name": "e3Id",
                "type": "uint256"
            },
            {
                "internalType": "uint256",
                "name": "identityCommitment",
                "type": "uint256"
            }
        ],
        "name": "registerMember",
        "outputs": [],
        "stateMutability": "nonpayable",
        "type": "function"
    },
    {
        "inputs": [
            {
                "internalType": "uint256",
                "name": "e3Id",
                "type": "uint256"
            }
        ],
        "name": "groupIds",
        "outputs": [
            {
                "internalType": "uint256",
                "name": "",
                "type": "uint256"
            }
        ],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [
            {
                "internalType": "uint256",
                "name": "groupId",
                "type": "uint256"
            },
            {
                "internalType": "uint256",
                "name": "identityCommitment",
                "type": "uint256"
            }
        ],
        "name": "committed",
        "outputs": [
            {
                "internalType": "bool",
                "name": "",
                "type": "bool"
            }
        ],
        "stateMutability": "view",
        "type": "function"
    },
    {
        "inputs": [
            {
                "internalType": "uint256",
                "name": "groupId",
                "type": "uint256"
            }
        ],
        "name": "getGroupCommitments",
        "outputs": [
            {
                "internalType": "uint256[]",
                "name": "",
                "type": "uint256[]"
            }
        ],
        "stateMutability": "view",
        "type": "function"
    }
] as const;

// Registry contract ABI - only the CommitteePublished event
export const REGISTRY_ABI = [
    {
        "anonymous": false,
        "inputs": [
            {
                "indexed": true,
                "internalType": "uint256",
                "name": "e3Id",
                "type": "uint256"
            },
            {
                "indexed": false,
                "internalType": "bytes",
                "name": "publicKey",
                "type": "bytes"
            }
        ],
        "name": "CommitteePublished",
        "type": "event"
    }
] as const;
