import { Identity } from '@semaphore-protocol/identity';
import { Group } from '@semaphore-protocol/group';
import { generateNoirProof as generateSemaphoreNoirProof, initSemaphoreNoirBackend, SemaphoreNoirProof } from '@semaphore-protocol/proof';

export type { SemaphoreNoirProof };
export { initSemaphoreNoirBackend };

export async function generateNoirProof(
    identity: Identity,
    group: Group,
    message: string,
    scope: string,
    backend: any,
    useKeccak: boolean = true
): Promise<SemaphoreNoirProof> {
    return generateSemaphoreNoirProof(
        identity,
        group,
        message,
        scope,
        backend,
        useKeccak
    );
}