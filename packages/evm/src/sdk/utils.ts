import { type Address, type Hash, type Log } from 'viem';

export class SDKError extends Error {
    constructor(message: string, public readonly code?: string) {
        super(message);
        this.name = 'SDKError';
    }
}

export function isValidAddress(address: string): address is Address {
    return /^0x[a-fA-F0-9]{40}$/.test(address);
}

export function isValidHash(hash: string): hash is Hash {
    return /^0x[a-fA-F0-9]{64}$/.test(hash);
}

export function formatEventName(contractName: string, eventName: string): string {
    return `${contractName}.${eventName}`;
}

export function parseEventData<T>(log: Log): T {
    // Parse the log data based on the event signature
    // This is a simplified version - in practice you'd decode the actual event data
    return log.data as unknown as T;
}

export function sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
}

export function formatBigInt(value: bigint): string {
    return value.toString();
}

export function parseBigInt(value: string): bigint {
    return BigInt(value);
}

export function generateEventId(log: Log): string {
    return `${log.blockHash}-${log.logIndex}`;
}

export function getCurrentTimestamp(): bigint {
    return BigInt(Math.floor(Date.now() / 1000));
} 