// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

const { ethers } = await network.connect();

/**
 * Encodes a fake pk_aggregation proof for testing when no real proof is available.
 * Format: abi.encode(bytes rawProof, bytes32[] publicInputs) with commitment as last input.
 * @param commitment The aggregate public key commitment (bytes32) as last public input.
 * @returns ABI-encoded proof bytes.
 */
export function encodePkProof(commitment: string): string {
  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  return abiCoder.encode(["bytes", "bytes32[]"], ["0x", [commitment]]);
}
