// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ZKInputsGenerator } from '@enclave/crisp-zk-inputs'
import { BFVParams } from './types'

export const CRISP_SERVER_TOKEN_TREE_ENDPOINT = 'state/token-holders'
export const CRISP_SERVER_STATE_LITE_ENDPOINT = 'state/lite'

/**
 * This is the maximum value for a vote (Yes or No). This is 2^28
 * The minimum degree that BFV should use is 56 (to accommodate both Yes and No votes)
 * If you change this value, make sure to update the circuit too.
 */
export const MAXIMUM_VOTE_VALUE = 268435456n

/**
 * Default BFV parameters for the CRISP ZK inputs generator.
 * These are the parameters used for the default testing purposes only.
 */
export const DEFAULT_BFV_PARAMS = ZKInputsGenerator.withDefaults().getBFVParams() as BFVParams

export const DEFAULT_HASHED_MESSAGE = [
  200, 232, 98, 162, 80, 131, 242, 57, 252, 76, 226, 45, 127, 206, 207, 39, 206, 44, 211, 171, 113, 67, 121, 68, 78, 253, 202, 79, 29, 128,
  130, 76,
]

export const DEFAULT_SIGNATURE = [
  22, 65, 67, 29, 14, 211, 253, 134, 129, 79, 2, 109, 166, 46, 17, 67, 75, 83, 198, 168, 81, 98, 254, 167, 249, 146, 24, 191, 60, 48, 125,
  236, 127, 54, 28, 35, 95, 7, 182, 88, 120, 10, 253, 145, 165, 201, 214, 141, 106, 75, 20, 213, 235, 5, 17, 246, 104, 141, 62, 145, 20, 14,
  236, 18,
]
export const DEFAULT_PUB_KEY_X = [
  131, 24, 83, 91, 84, 16, 93, 74, 122, 174, 96, 192, 143, 196, 95, 150, 135, 24, 27, 79, 223, 198, 37, 189, 26, 117, 63, 167, 57, 127, 237,
  117,
]

export const DEFAULT_PUB_KEY_Y = [
  53, 71, 241, 28, 168, 105, 102, 70, 242, 243, 172, 176, 142, 49, 1, 106, 250, 194, 62, 99, 12, 93, 17, 245, 159, 97, 254, 245, 123, 13,
  42, 165,
]
