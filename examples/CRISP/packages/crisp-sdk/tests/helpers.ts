// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { hashLeaf } from '../src/utils'

/**
 * Generate Merkle tree leaves for tests. Includes leaves for each (address, balance)
 * pair and pads with unique filler values to reach minSize.
 */
export const generateTestLeaves = (
  entries: { address: string; balance: bigint }[],
  minSize = 6,
): bigint[] => {
  const leaves = entries.map(({ address, balance }) => hashLeaf(address.toLowerCase(), balance))
  const fillerCount = Math.max(0, minSize - leaves.length)
  for (let i = 0; i < fillerCount; i++) {
    leaves.push(BigInt(1000 + i))
  }
  return leaves
}
