// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ZKInputsGenerator } from '@crisp-e3/zk-inputs'
import { BFVParams } from './types'

export const CRISP_SERVER_TOKEN_TREE_ENDPOINT = 'state/token-holders'
export const CRISP_SERVER_STATE_LITE_ENDPOINT = 'state/lite'

export const MERKLE_TREE_MAX_DEPTH = 20 // static, hardcoded in the circuit.

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
 * Mock message for masking signature
 */
export const MESSAGE = 'Vote for round 0'
