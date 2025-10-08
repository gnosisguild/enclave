// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { CRISP_SERVER_TOKEN_TREE_ENDPOINT } from './constants'

import { LeanIMT } from '@zk-kit/lean-imt'

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

export const hashLeaf = (address: string, balance: number) => {

}

/**
 * Generate a Merkle proof for a given address to prove inclusion in the voters' list
 */
export const generateMerkleProof = (threshold: number, balance: number, address: string) => {
  if (balance < threshold) {
    throw new Error('Balance is below the threshold')
  }




}

/**
 * Get the token balance at a specific block for a given address
 */
export const getBalanceAt = (tokenAddress: string, snapshotBlock: number) => {

}
