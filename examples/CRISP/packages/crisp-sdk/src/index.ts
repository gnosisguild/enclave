// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

export * from './token'
export * from './state'
export { MERKLE_TREE_MAX_DEPTH, SIGNATURE_MESSAGE, MAXIMUM_VOTE_VALUE } from './constants'
export { hashLeaf, generateMerkleProof, generateMerkleTree, getAddressFromSignature } from './utils'
export {
  decodeTally,
  generateVoteProof,
  generateMaskVoteProof,
  verifyProof,
  generatePublicKey,
  encryptVote,
  encodeSolidityProof,
} from './vote'

export type { RoundDetails, RoundDetailsResponse, TokenDetails, Vote, MaskVoteProofInputs, VoteProofInputs } from './types'
