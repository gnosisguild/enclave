// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

export const CRISP_SERVER_TOKEN_TREE_ENDPOINT = 'state/token-holders'
export const CRISP_SERVER_STATE_LITE_ENDPOINT = 'state/lite'

/**
 * This is the maximum value for a vote (Yes or No). This is 2^28
 * The minimum degree that BFV should use is 56 (to accommodate both Yes and No votes)
 * If you change this value, make sure to update the circuit too.
 */
export const MAXIMUM_VOTE_VALUE = 268435456n
