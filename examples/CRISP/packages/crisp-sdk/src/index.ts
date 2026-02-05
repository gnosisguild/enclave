// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

export * from './token'
export * from './state'
export { MERKLE_TREE_MAX_DEPTH, SIGNATURE_MESSAGE, SIGNATURE_MESSAGE_HASH } from './constants'
export { hashLeaf, generateMerkleProof, generateMerkleTree, getAddressFromSignature, getMaxVoteValue, getZeroVote } from './utils'
export {
  decodeTally,
  generateVoteProof,
  generateMaskVoteProof,
  verifyProof,
  generatePublicKey,
  encryptVote,
  encodeSolidityProof,
  validateVote,
} from './vote'
export { CrispSDK } from './sdk'

export type { RoundDetails, RoundDetailsResponse, TokenDetails, Vote, MaskVoteProofInputs, VoteProofInputs } from './types'
