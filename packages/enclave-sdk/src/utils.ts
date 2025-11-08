// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { type Address, type Hash, type Log, encodeAbiParameters } from "viem";

export class SDKError extends Error {
  constructor(
    message: string,
    public readonly code?: string,
  ) {
    super(message);
    this.name = "SDKError";
  }
}

export function isValidAddress(address: string): address is Address {
  return /^0x[a-fA-F0-9]{40}$/.test(address);
}

export function isValidHash(hash: string): hash is Hash {
  return /^0x[a-fA-F0-9]{64}$/.test(hash);
}

export function formatEventName(
  contractName: string,
  eventName: string,
): string {
  return `${contractName}.${eventName}`;
}

export function parseEventData<T>(log: Log): T {
  return log.data as unknown as T;
}

/**
 * Sleep for a specified number of milliseconds
 */
export const sleep = (ms: number): Promise<void> => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};

export function formatBigInt(value: bigint): string {
  return value.toString();
}

export function parseBigInt(value: string): bigint {
  return BigInt(value);
}

export function generateEventId(log: Log): string {
  return `${log.blockHash}-${log.logIndex}`;
}

/**
 * Get the current timestamp in seconds
 */
export function getCurrentTimestamp(): number {
  return Math.floor(Date.now() / 1000);
}

// BFV parameter set matching the Rust SET_2048_1032193_1 configuration
export const SET_2048_1032193_1 = {
  degree: 2048,
  plaintext_modulus: 1032193,
  moduli: [0x3fffffff000001n], // BigInt for the modulus
  error2_variance: "10",
} as const;

// BFV parameter set matching the Rust SET_8192_1000_4 configuration
export const SET_8192_1000_4 = {
  degree: 8192,
  plaintext_modulus: 1000,
  moduli: [
    0x00800000022a0001n,
    0x00800000021a0001n,
    0x0080000002120001n,
    0x0080000001f60001n,
  ],
  error2_variance:
    "52309181128222339698631578526730685514457152477762943514050560000",
};

// Set default parameter set
export const BFV_PARAMS_SET = SET_2048_1032193_1;

// Compute provider parameters structure
export interface ComputeProviderParams {
  name: string;
  parallel: boolean;
  batch_size: number;
}

// Default compute provider configuration
export const DEFAULT_COMPUTE_PROVIDER_PARAMS: ComputeProviderParams = {
  name: "risc0",
  parallel: false,
  batch_size: 2,
};

// Default E3 configuration
export const DEFAULT_E3_CONFIG = {
  threshold_min: 2,
  threshold_max: 5,
  window_size: 120, // 2 minutes in seconds
  duration: 1800, // 30 minutes in seconds
  payment_amount: "0", // 0 ETH in wei
} as const;

/**
 * Encode BFV parameters for the smart contract
 * BFV (Brakerski-Fan-Vercauteren) is a type of fully homomorphic encryption
 */
export function encodeBfvParams(
  degree: number = BFV_PARAMS_SET.degree,
  plaintext_modulus: number = BFV_PARAMS_SET.plaintext_modulus,
  moduli: readonly bigint[] = BFV_PARAMS_SET.moduli,
  error2_variance: string = BFV_PARAMS_SET.error2_variance,
): `0x${string}` {
  return encodeAbiParameters(
    [
      {
        name: "bfvParams",
        type: "tuple",
        components: [
          { name: "degree", type: "uint256" },
          { name: "plaintext_modulus", type: "uint256" },
          { name: "moduli", type: "uint256[]" },
          { name: "error2_variance", type: "string" },
        ],
      },
    ],
    [
      {
        degree: BigInt(degree),
        plaintext_modulus: BigInt(plaintext_modulus),
        moduli: [...moduli],
        error2_variance,
      },
    ],
  );
}

/**
 * Encode compute provider parameters for the smart contract'
 * If mock is true, the compute provider parameters will return 32 bytes of 0x00
 */
export function encodeComputeProviderParams(
  params: ComputeProviderParams,
  mock: boolean = false,
): `0x${string}` {
  if (mock) {
    return `0x${"0".repeat(32)}` as `0x${string}`;
  }

  const jsonString = JSON.stringify(params);
  const encoder = new TextEncoder();
  const bytes = encoder.encode(jsonString);

  return `0x${Array.from(bytes, (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("")}`;
}

/**
 * Encode custom parameters for the smart contract.
 */
export function encodeCustomParams(
  params: Record<string, unknown>,
): `0x${string}` {
  const jsonString = JSON.stringify(params);
  const encoder = new TextEncoder();
  const bytes = encoder.encode(jsonString);

  return `0x${Array.from(bytes, (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("")}`;
}

/**
 * Calculate start window for E3 request
 */
export function calculateStartWindow(
  windowSize: number = DEFAULT_E3_CONFIG.window_size,
): [bigint, bigint] {
  const now = getCurrentTimestamp();
  return [BigInt(now), BigInt(now + windowSize)];
}

/**
 * Decode plaintextOutput bytes to get the actual result number
 */
export function decodePlaintextOutput(plaintextOutput: string): number | null {
  try {
    // Remove '0x' prefix if present
    const hex = plaintextOutput.startsWith("0x")
      ? plaintextOutput.slice(2)
      : plaintextOutput;

    // Convert hex to bytes
    const bytes = new Uint8Array(
      hex.match(/.{1,2}/g)?.map((byte) => parseInt(byte, 16)) || [],
    );

    if (bytes.length < 8) {
      console.warn("Plaintext output too short for u64 decoding");
      return null;
    }

    // Decode first u64 (8 bytes) as little-endian
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const result = view.getBigUint64(0, true); // true for little-endian

    return Number(result);
  } catch (error) {
    console.error("Failed to decode plaintext output:", error);
    return null;
  }
}
