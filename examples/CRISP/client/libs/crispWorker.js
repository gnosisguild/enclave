// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { encryptVote, generateMerkleProof, hashLeaf, generateVoteProof, encodeSolidityProof } from '@crisp-e3/sdk'

self.onmessage = async function (event) {
  const { type, data } = event.data
  switch (type) {
    case 'generate_proof':
      try {
        const { voteId, publicKey, address, signature } = data

        // voteId is either 0 or 1, so we need to encode the vote accordingly.
        // We are adapting to the current CRISP application.
        const vote = voteId === 0 ? { yes: 0n, no: 1n } : { yes: 1n, no: 0n }
        const balance = 1n

        const leaf = hashLeaf(address, balance)
        // todo: get the leaves from the server (pass them from the client).
        const merkleProof = generateMerkleProof(balance, address, [
          leaf,
          4720511075913887710172192848636076523165432993226978491435561065722130431597n,
          14131255645332550266535358189863475289290770471998199141522479556687499890181n,
        ])

        const encryptedVote = encryptVote(vote, publicKey)
        const proof = await generateVoteProof({
          vote,
          publicKey,
          previousCiphertext: encryptedVote,
          signature,
          merkleProof,
          balance,
          slotAddress: address,
        })

        const encodedProof = encodeSolidityProof(proof)

        self.postMessage({
          type: 'generate_proof',
          success: true,
          encodedProof,
        })
      } catch (error) {
        self.postMessage({ type: 'generate_proof', success: false, error: error.message })
      }
      break

    default:
      console.error(`Unknown message type: ${type}`)
  }
}
