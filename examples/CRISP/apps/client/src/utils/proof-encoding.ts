import { hexToBytes, encodeAbiParameters, parseAbiParameters } from 'viem';
import { type SemaphoreProof } from '@semaphore-protocol/core';
import { type SemaphoreNoirProof } from '@/utils/semaphoreNoirProof';

const abi = parseAbiParameters(
    'uint256,uint256,uint256,uint256,uint256,uint256[8]'
);

const noirAbi = parseAbiParameters(
    'uint256,uint256,uint256,uint256,uint256,bytes'
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

export function encodeSemaphoreNoirProof(
    { merkleTreeDepth, merkleTreeRoot, nullifier, message, scope, proofBytes }: SemaphoreNoirProof
): Uint8Array {
    const hex = encodeAbiParameters(noirAbi, [
        BigInt(merkleTreeDepth),
        BigInt(merkleTreeRoot),
        BigInt(nullifier),
        BigInt(message),
        BigInt(scope),
        `0x${Buffer.from(proofBytes).toString('hex')}` as `0x${string}`,
    ]);

    return hexToBytes(hex);
}