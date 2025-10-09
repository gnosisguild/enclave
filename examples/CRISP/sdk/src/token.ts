// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { CRISP_SERVER_TOKEN_TREE_ENDPOINT } from './constants'

/**
 * Get the merkle tree data from the CRISP server
 * @param serverUrl - The base URL of the CRISP server
 * @param e3Id - The e3Id of the round
 */
export const getTreeData = async (serverUrl: string, e3Id: number) => {
  const response = await fetch(`${serverUrl}/${CRISP_SERVER_TOKEN_TREE_ENDPOINT}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ round_id: e3Id }),
  })

  const hashes = await response.json()

  return hashes
}

/**
 * Generate a Merkle proof for a given address to prove inclusion in the voters' list
 */
export const generateMerkleProof = () => {}

/**
 * Get the token balance at a specific block for a given address
 */
export const getBalanceAt = () => {}
