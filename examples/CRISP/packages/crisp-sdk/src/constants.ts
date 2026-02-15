// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { hashMessage } from 'viem'

export const CRISP_SERVER_TOKEN_TREE_ENDPOINT = 'state/token-holders'
export const CRISP_SERVER_STATE_LITE_ENDPOINT = 'state/lite'
export const CRISP_SERVER_PREVIOUS_CIPHERTEXT_ENDPOINT = 'state/previous-ciphertext'
export const CRISP_SERVER_IS_SLOT_EMPTY_ENDPOINT = 'state/is-slot-empty'

export const MERKLE_TREE_MAX_DEPTH = 20 // static, hardcoded in the circuit.

// @note that the following must be changed accordingly to the CRISP circuit
// Hard limit on the maximum number of vote bits supported for each option.
export const MAX_VOTE_BITS = 50
// Hard limit on the maximum number of vote options supported.
export const MAX_VOTE_OPTIONS = 10

/**
 * Message used by users to prove ownership of their Ethereum account
 * This message is signed by the user's private key to authenticate their identity
 * @notice Apps ideally want to use a different message to avoid signature reuse across different applications
 */
export const SIGNATURE_MESSAGE = 'CRISP: Sign this message to prove ownership of your Ethereum account'
export const SIGNATURE_MESSAGE_HASH = hashMessage(SIGNATURE_MESSAGE)

// Placeholder signature for masking votes.
export const MASK_SIGNATURE =
  '0x8e7d77112641d59e9409ec3052041703bb9d9e6ed39bfcf75aefbcafe829ac6b21dd7648116ad5db0466fcb4bd468dcb28f6c069def8bc47cd9d859c85a016e31b'
