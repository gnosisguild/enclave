// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

const E3_PROGRAM_ADDRESS_FROM_ENV = import.meta.env.VITE_E3_PROGRAM_ADDRESS;

if (!E3_PROGRAM_ADDRESS_FROM_ENV) {
    throw new Error("VITE_E3_PROGRAM_ADDRESS environment variable is not set.");
}

export const E3_PROGRAM_ADDRESS = E3_PROGRAM_ADDRESS_FROM_ENV;

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
