// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { encodeAbiParameters, parseAbiParameters, bytesToHex } from 'viem'

const crispAbi = parseAbiParameters('(bytes, bytes32[], bytes)')

export const encodeCrispInputs = (noirProof: Uint8Array, noirPublicInputs: string[], encryptedVote: Uint8Array): string => {
  return encodeAbiParameters(crispAbi, [
    [bytesToHex(noirProof), noirPublicInputs.map((input) => input as `0x${string}`), bytesToHex(encryptedVote)],
  ])
}
