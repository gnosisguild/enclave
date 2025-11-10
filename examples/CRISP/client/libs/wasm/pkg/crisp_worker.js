// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import {
  encryptVoteAndGenerateCRISPInputs,
  generateProofWithReturnValue,
  VotingMode,
  encodeVote,
  encryptVote,
  generateMerkleProof,
  verifyProof,
  hashLeaf,
} from '@crisp-e3/sdk'

self.onmessage = async function (event) {
  const { type, data } = event.data
  switch (type) {
    case 'encrypt_vote':
      try {
        const { voteId, publicKey, address, signature, message } = data

        // voteId is either 0 or 1, so we need to encode the vote accordingly.
        // We are adapting to the current CRISP application.
        const vote = voteId === 0 ? { yes: 0n, no: 1n } : { yes: 1n, no: 0n }
        const balance = 1n

        const leaf = hashLeaf(address.toLowerCase(), balance.toString())
        // TODO: get the leaves from the server (pass them from the client).
        const merkleProof = generateMerkleProof(0n, balance, address, [
          leaf,
          4720511075913887710172192848636076523165432993226978491435561065722130431597n,
          14131255645332550266535358189863475289290770471998199141522479556687499890181n,
        ])

        const encodedVote = encodeVote(vote, VotingMode.GOVERNANCE, balance)
        const encryptedVote = await encryptVote(encodedVote, publicKey)

        const inputs = await encryptVoteAndGenerateCRISPInputs({
          encodedVote,
          publicKey,
          previousCiphertext: encryptedVote,
          signature,
          message,
          merkleData: merkleProof,
          balance,
          slotAddress: address.toLowerCase(),
          isFirstVote: true,
        })

        const { proof, returnValue } = await generateProofWithReturnValue(inputs)

        // TODO: returnValue is the encrypted vote. We need to convert it from Noir format to BFV format
        // instead of using the encryptVote function (which should be removed from the SDK).

        self.postMessage({
          type: 'encrypt_vote',
          success: true,
          encryptedVote: {
            vote: encryptedVote,
            proofData: proof,
          },
        })
      } catch (error) {
        self.postMessage({ type: 'encrypt_vote', success: false, error: error.message })
      }
      break

    default:
      console.error(`Unknown message type: ${type}`)
  }
}
