// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { encodeSolidityProof, CrispSDK } from '@crisp-e3/sdk'

self.onmessage = async function (event) {
  const { type, data } = event.data
  switch (type) {
    case 'generate_proof':
      try {
        const { e3Id, vote, publicKey, balance, address: slotAddress, signature, messageHash, isMasking, crispServer, merkleLeaves } = data

        const sdk = new CrispSDK(crispServer)

        let proof

        if (isMasking) {
          proof = await sdk.generateMaskVoteProof({
            e3Id,
            publicKey,
            balance,
            slotAddress,
            merkleLeaves,
          })
        } else {
          proof = await sdk.generateVoteProof({
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
