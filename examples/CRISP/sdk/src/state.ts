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
    e3Id: Number(data.id),
    tokenAddress: data.token_address,
    balanceThreshold: data.balance_threshold,
    chainId: Number(data.chain_id),
    enclaveAddress: data.enclave_address,
    status: data.status,
    voteCount: Number(data.vote_count),
    startTime: Number(data.start_time),
    duration: Number(data.duration),
    expiration: Number(data.expiration),
    startBlock: Number(data.start_block),
    committeePublicKey: data.committee_public_key,
    emojis: data.emojis,
    snapshotBlock: Number(data.start_block),
  }
}

/**
 * Get the token address and balance threshold for a specific round
 * @param serverUrl - The base URL of the CRISP server
 * @param e3Id - The e3Id of the round
 * @returns The token address and balance threshold
 */
export const getRoundTokenAndThreshold = async (serverUrl: string, e3Id: number): Promise<ITokenDetails> => {
  const roundDetails = await getRoundDetails(serverUrl, e3Id)
  return {
    tokenAddress: roundDetails.tokenAddress,
    threshold: roundDetails.balanceThreshold,
  }
}
