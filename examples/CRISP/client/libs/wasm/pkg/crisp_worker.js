// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { EnclaveSDK, FheProtocol } from '@enclave-e3/sdk'
import circuit from '../../noir/crisp_circuit.json'

self.onmessage = async function (event) {
  const { type, data } = event.data
  switch (type) {
    case 'encrypt_vote':
      try {
        const { voteId, publicKey } = data
        // use default params for now as they do not matter for what we are doing here,
        // which is just encrypting the vote and generating a proof
        const sdk = EnclaveSDK.create({
          chainId: 31337,
          contracts: {
            enclave: '0xc6e7DF5E7b4f2A278906862b61205850344D4e7d',
            ciphernodeRegistry: '0xc6e7DF5E7b4f2A278906862b61205850344D4e7d',
          },
          // local node
          rpcUrl: 'http://localhost:8545',
          // default Anvil private key
          privateKey: '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80',
          protocol: FheProtocol.BFV,
        })

        const result = await sdk.encryptNumberAndGenProof(voteId, publicKey, circuit)

        self.postMessage({
          type: 'encrypt_vote',
          success: true,
          encryptedVote: {
            vote: result.encryptedVote,
            proofData: result.proof,
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
