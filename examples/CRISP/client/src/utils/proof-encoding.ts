// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { hexToBytes, encodeAbiParameters, parseAbiParameters, bytesToHex } from 'viem';
import { type SemaphoreNoirProof } from '@hashcloak/semaphore-noir-proof';

const abi = parseAbiParameters(
    '(uint256,uint256,uint256,uint256,uint256,bytes)'
);

export function encodeSemaphoreProof(
    { merkleTreeDepth, merkleTreeRoot, nullifier, message, scope, proofBytes }: SemaphoreNoirProof
): Uint8Array {
    const hex = encodeAbiParameters(abi, [
        [
            BigInt(merkleTreeDepth),
            BigInt(merkleTreeRoot),
            BigInt(nullifier),
            BigInt(message),
            BigInt(scope),
            bytesToHex(proofBytes),
        ]
    ]);

    return hexToBytes(hex);
}
