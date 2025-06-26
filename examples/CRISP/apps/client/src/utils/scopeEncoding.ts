/**
 * Utility functions for encoding and decoding scope values for CRISP contracts.
 * 
 * The scope encodes both the subject address and group ID to prevent front-running attacks:
 * - Upper 160 bits: Subject address (20 bytes)  
 * - Lower 96 bits: Group ID (12 bytes)
 */

/**
 * Encodes an address and group ID into a scope value
 * @param address The subject address (20 bytes, 160 bits)
 * @param groupId The group ID (up to 96 bits)
 * @returns The encoded scope as a BigInt
 */
export function encodeScope(address: string, groupId: bigint): bigint {
    // Remove 0x prefix if present
    const cleanAddress = address.startsWith('0x') ? address.slice(2) : address;
    
    // Convert address to BigInt (160 bits)
    const addressBigInt = BigInt('0x' + cleanAddress);
    
    // Ensure groupId fits in 96 bits
    const maxGroupId = (1n << 96n) - 1n;
    if (groupId > maxGroupId) {
        throw new Error(`Group ID ${groupId} exceeds maximum value of ${maxGroupId}`);
    }
    
    // Encode: (address << 96) | groupId
    return (addressBigInt << 96n) | groupId;
}

/**
 * Decodes a scope value into address and group ID
 * @param scope The encoded scope value
 * @returns Object with address and groupId
 */
export function decodeScope(scope: bigint): { address: string, groupId: bigint } {
    // Extract group ID (lower 96 bits)
    const groupId = scope & ((1n << 96n) - 1n);
    
    // Extract address (upper 160 bits)
    const addressBigInt = scope >> 96n;
    
    // Convert back to hex address
    const address = '0x' + addressBigInt.toString(16).padStart(40, '0');
    
    return { address, groupId };
}

/**
 * Validates that a scope contains the expected address and group ID
 * @param scope The scope to validate
 * @param expectedAddress The expected address
 * @param expectedGroupId The expected group ID
 * @returns True if scope matches expectations
 */
export function validateScope(scope: bigint, expectedAddress: string, expectedGroupId: bigint): boolean {
    const decoded = decodeScope(scope);
    return decoded.address.toLowerCase() === expectedAddress.toLowerCase() && 
           decoded.groupId === expectedGroupId;
} 