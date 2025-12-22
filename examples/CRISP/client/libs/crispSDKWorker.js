// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { hashLeaf, encodeSolidityProof, CrispSDK } from '@crisp-e3/sdk'

self.onmessage = async function (event) {
  const { type, data } = event.data
  switch (type) {
    case 'generate_proof':
      try {
        const { e3Id, vote, publicKey, balance, address: slotAddress, signature, messageHash, isMasking, crispServer } = data

        // todo: get the leaves from the server (pass them from the client).
        const merkleLeaves = [
          hashLeaf(slotAddress, balance),
          4720511075913887710172192848636076523165432993226978491435561065722130431597n,
          14131255645332550266535358189863475289290770471998199141522479556687499890181n,
        ]

        const sdk = new CrispSDK(crispServer)

        let proof

        if (isMasking) {
          proof = await sdk.generateMaskVoteProof({
            serverUrl: crispServer,
            e3Id,
            publicKey,
            balance,
            slotAddress,
            merkleLeaves,
          })
        } else {
          proof = await sdk.generateVoteProof({
            serverUrl: crispServer,
            vote,
            e3Id,
            publicKey,
            signature,
            merkleLeaves,
            balance,
            messageHash,
            slotAddress,
          })
        }

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
