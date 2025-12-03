// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import os from 'os'
import { hashMessage } from 'viem'

export const CRISP_SERVER_TOKEN_TREE_ENDPOINT = 'state/token-holders'
export const CRISP_SERVER_STATE_LITE_ENDPOINT = 'state/lite'

export const MERKLE_TREE_MAX_DEPTH = 20 // static, hardcoded in the circuit.

/**
 * Optimal number of threads for proof generation
 * Leaves at least 1 core free for other operations
 */
export const OPTIMAL_THREAD_COUNT = Math.max(
  1,
  (typeof os.availableParallelism === 'function' ? os.availableParallelism() : os.cpus().length) - 1,
)

/**
 * Half the minimum degree needed to support the maxium vote value
 * If you change MAXIMUM_VOTE_VALUE, make sure to update this value too.
 */
export const HALF_LARGEST_MINIMUM_DEGREE = 28

/**
 * This is the maximum value for a vote (Yes or No). This is 2^28 - 1
 * The minimum degree that BFV should use is 56 (to accommodate both Yes and No votes)
 */
export const MAXIMUM_VOTE_VALUE = BigInt(Math.pow(2, HALF_LARGEST_MINIMUM_DEGREE) - 1)

/**
 * Message used by users to prove ownership of their Ethereum account
 * This message is signed by the user's private key to authenticate their identity
 */
export const SIGNATURE_MESSAGE = 'CRISP: Sign this message to prove ownership of your Ethereum account'
export const SIGNATURE_MESSAGE_HASH = hashMessage(SIGNATURE_MESSAGE)

// Placeholder signature for masking votes.
export const MASK_SIGNATURE =
  '0x8e7d77112641d59e9409ec3052041703bb9d9e6ed39bfcf75aefbcafe829ac6b21dd7648116ad5db0466fcb4bd468dcb28f6c069def8bc47cd9d859c85a016e31b'
