export const E3_PROGRAM_ADDRESS = '0xc6e7DF5E7b4f2A278906862b61205850344D4e7d';

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
    }
] as const;