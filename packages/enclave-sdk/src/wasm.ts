// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { bfv_encrypt_number, bfv_verifiable_encrypt_number } from "@gnosis-guild/e3-wasm";
import initializeWasm from "@gnosis-guild/e3-wasm/init";

import type { BfvVerifiableEncryptionResult } from "./types";

/**
 * Encrypt a number using the BFV scheme
 * @param data - The number to encrypt
 * @param publicKey - The public key to use for encryption
 * @returns The encrypted number
 */
export async function bfvEncryptNumber(
  data: bigint,
  publicKey: Uint8Array,
): Promise<Uint8Array> {
  await initializeWasm();
  return bfv_encrypt_number(data, publicKey);
}

/**
 * Encrypt a number using the BFV scheme and generate circuit inputs for Greco
 * @param data - The number to encrypt
 * @param publicKey - The public key to use for encryption
 * @returns The encrypted number and circuit inputs
 */
export async function bfvVerifiableEncryptNumber(
  data: bigint,
  publicKey: Uint8Array,
): Promise<BfvVerifiableEncryptionResult> {
  await initializeWasm();
  const [encryptedVote, circuitInputs] = bfv_verifiable_encrypt_number(data, publicKey);
  
  return {
    encryptedVote,
    circuitInputs,
  };
}
