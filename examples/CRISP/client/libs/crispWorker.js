// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { hashLeaf, generateVoteProof, encodeSolidityProof } from '@crisp-e3/sdk'

self.onmessage = async function (event) {
  const { type, data } = event.data
  switch (type) {
    case 'generate_proof':
      try {
        const { voteId, publicKey, address, signature, previousCiphertext, messageHash } = data

        // voteId is either 0 or 1, so we need to encode the vote accordingly.
        // We are adapting to the current CRISP application.
        const balance = 1n
        const vote = voteId === 0 ? { yes: 0n, no: balance } : { yes: balance, no: 0n }

        // todo: get the leaves from the server (pass them from the client).
        const merkleLeaves = [
          hashLeaf(address, balance),
          4720511075913887710172192848636076523165432993226978491435561065722130431597n,
          14131255645332550266535358189863475289290770471998199141522479556687499890181n,
        ]

        const proof = await generateVoteProof({
          vote,
          publicKey,
          signature,
          merkleLeaves,
          balance,
          previousCiphertext,
          messageHash,
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
