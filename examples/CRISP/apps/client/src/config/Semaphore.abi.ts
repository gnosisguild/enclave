const ENV_SEMAPHORE_ADDRESS = import.meta.env.VITE_SEMAPHORE_ADDRESS;

if (!ENV_SEMAPHORE_ADDRESS) {
    throw new Error("VITE_SEMAPHORE_ADDRESS environment variable is not set.");
}

export const SEMAPHORE_ADDRESS = ENV_SEMAPHORE_ADDRESS;
export const SEMAPHORE_ABI = [
    {
        anonymous: false,
        name: 'MemberAdded',
        type: 'event',
        inputs: [
            { indexed: true, name: 'groupId', type: 'uint256' },
            { indexed: true, name: 'index', type: 'uint256' },
            { indexed: false, name: 'identityCommitment', type: 'uint256' },
            { indexed: false, name: 'merkleTreeRoot', type: 'uint256' },
        ],
    },
] as const;