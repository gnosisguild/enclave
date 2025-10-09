// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { CRISP_SERVER_STATE_LITE_ENDPOINT } from './constants'

import type { IRoundDetailsResponse, IRoundDetails, ITokenDetails } from './types'

/**
 * Get the details of a specific round
 */
export const getRoundDetails = async (serverUrl: string, e3Id: number): Promise<IRoundDetails> => {
  const response = await fetch(`${serverUrl}/${CRISP_SERVER_STATE_LITE_ENDPOINT}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ round_id: e3Id }),
  })

  const data = (await response.json()) as IRoundDetailsResponse

  return {
    e3Id: BigInt(data.id),
    tokenAddress: data.token_address,
    balanceThreshold: BigInt(data.balance_threshold),
    chainId: BigInt(data.chain_id),
    enclaveAddress: data.enclave_address,
    status: data.status,
    voteCount: BigInt(data.vote_count),
    startTime: BigInt(data.start_time),
    duration: BigInt(data.duration),
    expiration: BigInt(data.expiration),
    startBlock: BigInt(data.start_block),
    committeePublicKey: data.committee_public_key,
    emojis: data.emojis,
  }
}

/**
 * Get the token address, balance threshold and snapshot block for a specific round
 * @param serverUrl - The base URL of the CRISP server
 * @param e3Id - The e3Id of the round
 * @returns The token address, balance threshold and snapshot block
 */
export const getRoundTokenDetails = async (serverUrl: string, e3Id: number): Promise<ITokenDetails> => {
  const roundDetails = await getRoundDetails(serverUrl, e3Id)
  return {
    tokenAddress: roundDetails.tokenAddress,
    threshold: roundDetails.balanceThreshold,
    snapshotBlock: roundDetails.startBlock,
  }
}
