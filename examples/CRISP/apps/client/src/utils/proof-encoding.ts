import { hexToBytes, encodeAbiParameters, parseAbiParameters } from 'viem';
import { type SemaphoreProof } from '@semaphore-protocol/core';

const abi = parseAbiParameters(
    'uint256,uint256,uint256,uint256,uint256,uint256[8]'
);

type Tuple8<T> = readonly [T, T, T, T, T, T, T, T];

export function encodeSemaphoreProof(
    { merkleTreeDepth, merkleTreeRoot, nullifier, message, scope, points }: SemaphoreProof
): Uint8Array {
    if (points.length !== 8) {
        throw new Error('Semaphore proof must have 8 points');
    }

    const hex = encodeAbiParameters(abi, [
        BigInt(merkleTreeDepth),
        BigInt(merkleTreeRoot),
        BigInt(nullifier),
        BigInt(message),
        BigInt(scope),
        points.map(BigInt) as unknown as Tuple8<bigint>,
    ]);

    return hexToBytes(hex);
}
