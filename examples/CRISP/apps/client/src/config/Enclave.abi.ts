const E3_PROGRAM_ADDRESS_FROM_ENV = import.meta.env.VITE_E3_PROGRAM_ADDRESS;
const ENCLAVE_ADDRESS_FROM_ENV = import.meta.env.VITE_ENCLAVE_ADDRESS;
const FILTER_REGISTRY_ADDRESS_FROM_ENV = import.meta.env.VITE_FILTER_REGISTRY_ADDRESS;

// Check for missing environment variables
const missingEnvVars: string[] = [];

if (!E3_PROGRAM_ADDRESS_FROM_ENV) {
    missingEnvVars.push("VITE_E3_PROGRAM_ADDRESS");
}

if (!ENCLAVE_ADDRESS_FROM_ENV) {
    missingEnvVars.push("VITE_ENCLAVE_ADDRESS");
}

if (!FILTER_REGISTRY_ADDRESS_FROM_ENV) {
    missingEnvVars.push("VITE_FILTER_REGISTRY_ADDRESS");
}

export const MISSING_ENV_VARS = missingEnvVars;
export const HAS_MISSING_ENV_VARS = missingEnvVars.length > 0;

export const E3_PROGRAM_ADDRESS = E3_PROGRAM_ADDRESS_FROM_ENV || "";
export const ENCLAVE_ADDRESS = ENCLAVE_ADDRESS_FROM_ENV || "";
export const FILTER_REGISTRY_ADDRESS = FILTER_REGISTRY_ADDRESS_FROM_ENV || "";

export const ENCLAVE_ABI = [
    {
        "inputs": [
            {
                "internalType": "address",
                "name": "filter",
                "type": "address"
            },
            {
                "internalType": "uint32[2]",
                "name": "threshold",
                "type": "uint32[2]"
            },
            {
                "internalType": "uint256[2]",
                "name": "startWindow",
                "type": "uint256[2]"
            },
            {
                "internalType": "uint256",
                "name": "duration",
                "type": "uint256"
            },
            {
                "internalType": "address",
                "name": "e3Program",
                "type": "address"
            },
            {
                "internalType": "bytes",
                "name": "e3ProgramParams",
                "type": "bytes"
            },
            {
                "internalType": "bytes",
                "name": "computeProviderParams",
                "type": "bytes"
            }
        ],
        "name": "request",
        "outputs": [
            {
                "internalType": "uint256",
                "name": "e3Id",
                "type": "uint256"
            },
            {
                "components": [
                    {
                        "internalType": "uint256",
                        "name": "seed",
                        "type": "uint256"
                    },
                    {
                        "internalType": "uint32[2]",
                        "name": "threshold",
                        "type": "uint32[2]"
                    },
                    {
                        "internalType": "uint256",
                        "name": "requestBlock",
                        "type": "uint256"
                    },
                    {
                        "internalType": "uint256[2]",
                        "name": "startWindow",
                        "type": "uint256[2]"
                    },
                    {
                        "internalType": "uint256",
                        "name": "duration",
                        "type": "uint256"
                    },
                    {
                        "internalType": "uint256",
                        "name": "expiration",
                        "type": "uint256"
                    },
                    {
                        "internalType": "bytes32",
                        "name": "encryptionSchemeId",
                        "type": "bytes32"
                    },
                    {
                        "internalType": "address",
                        "name": "e3Program",
                        "type": "address"
                    },
                    {
                        "internalType": "bytes",
                        "name": "e3ProgramParams",
                        "type": "bytes"
                    },
                    {
                        "internalType": "address",
                        "name": "inputValidator",
                        "type": "address"
                    },
                    {
                        "internalType": "address",
                        "name": "decryptionVerifier",
                        "type": "address"
                    },
                    {
                        "internalType": "bytes32",
                        "name": "committeePublicKey",
                        "type": "bytes32"
                    },
                    {
                        "internalType": "bytes32",
                        "name": "ciphertextOutput",
                        "type": "bytes32"
                    },
                    {
                        "internalType": "bytes",
                        "name": "plaintextOutput",
                        "type": "bytes"
                    }
                ],
                "internalType": "struct E3",
                "name": "e3",
                "type": "tuple"
            }
        ],
        "stateMutability": "payable",
        "type": "function"
    },
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
                "components": [
                    {
                        "internalType": "uint256",
                        "name": "seed",
                        "type": "uint256"
                    },
                    {
                        "internalType": "uint32[2]",
                        "name": "threshold",
                        "type": "uint32[2]"
                    },
                    {
                        "internalType": "uint256",
                        "name": "requestBlock",
                        "type": "uint256"
                    },
                    {
                        "internalType": "uint256[2]",
                        "name": "startWindow",
                        "type": "uint256[2]"
                    },
                    {
                        "internalType": "uint256",
                        "name": "duration",
                        "type": "uint256"
                    },
                    {
                        "internalType": "uint256",
                        "name": "expiration",
                        "type": "uint256"
                    },
                    {
                        "internalType": "bytes32",
                        "name": "encryptionSchemeId",
                        "type": "bytes32"
                    },
                    {
                        "internalType": "address",
                        "name": "e3Program",
                        "type": "address"
                    },
                    {
                        "internalType": "bytes",
                        "name": "e3ProgramParams",
                        "type": "bytes"
                    },
                    {
                        "internalType": "address",
                        "name": "inputValidator",
                        "type": "address"
                    },
                    {
                        "internalType": "address",
                        "name": "decryptionVerifier",
                        "type": "address"
                    },
                    {
                        "internalType": "bytes32",
                        "name": "committeePublicKey",
                        "type": "bytes32"
                    },
                    {
                        "internalType": "bytes32",
                        "name": "ciphertextOutput",
                        "type": "bytes32"
                    },
                    {
                        "internalType": "bytes",
                        "name": "plaintextOutput",
                        "type": "bytes"
                    }
                ],
                "indexed": false,
                "internalType": "struct E3",
                "name": "e3",
                "type": "tuple"
            },
            {
                "indexed": true,
                "internalType": "address",
                "name": "filter",
                "type": "address"
            },
            {
                "indexed": true,
                "internalType": "address",
                "name": "e3Program",
                "type": "address"
            }
        ],
        "name": "E3Requested",
        "type": "event"
    },
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
                "internalType": "uint256",
                "name": "expiresAt",
                "type": "uint256"
            },
            {
                "indexed": false,
                "internalType": "bytes",
                "name": "publicKey",
                "type": "bytes"
            }
        ],
        "name": "E3Activated",
        "type": "event"
    }
] as const;

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
