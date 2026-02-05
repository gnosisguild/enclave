// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { zeroAddress } from 'viem'
import {
  hashLeaf,
  generatePublicKey,
  SIGNATURE_MESSAGE,
  generateVoteProof,
  getAddressFromSignature,
  encodeSolidityProof,
  generateMerkleTree,
  SIGNATURE_MESSAGE_HASH,
  generateMaskVoteProof,
} from '@crisp-e3/sdk'
import { expect } from 'chai'
import { deployCRISPProgram, deployHonkVerifier, deployMockEnclave, ethers } from './utils'
import { AbiCoder } from 'ethers'

let publicKey = generatePublicKey()

describe('CRISP Contracts', function () {
  describe('decode tally', () => {
    it('should decode a tally correctly', async () => {
      const mockEnclave = await deployMockEnclave()
      const crispProgram = await deployCRISPProgram({ mockEnclave })

      await mockEnclave.request(await crispProgram.getAddress())

      const tally =
        '0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000001000000000000000000000000000000010000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000100000000000000010000000000000001000000000000000100000000000000010000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000300000000000000000000000000000003000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000030000000000000003000000000000000300000000000000030000000000000003000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000'

      await mockEnclave.setPlaintextOutput(tally)

      const decodedTally1 = await crispProgram.decodeTally(0)

      expect(decodedTally1[0]).to.equal(10000000000n)
      expect(decodedTally1[1]).to.equal(30000000000n)
    })
  })

  describe('validate input', () => {
    it('should verify the proof correctly with the crisp verifier', async function () {
      // It needs some time to generate the proof.
      this.timeout(60000)

      const honkVerifier = await deployHonkVerifier()
      const [signer] = await ethers.getSigners()

      const vote = [10n, 0n]
      const balance = 100n
      const signature = (await signer.signMessage(SIGNATURE_MESSAGE)) as `0x${string}`
      const address = await getAddressFromSignature(signature, SIGNATURE_MESSAGE_HASH)
      const leaves = [...[10n, 20n, 30n], hashLeaf(address, balance)]

      const proof = await generateVoteProof({
        vote,
        publicKey,
        signature,
        merkleLeaves: leaves,
        balance,
        messageHash: SIGNATURE_MESSAGE_HASH,
        slotAddress: address,
      })

      const isValid = await honkVerifier.verify(proof.proof, proof.publicInputs)

      expect(isValid).to.be.true
    })

    it('should verify the proof for a vote mask', async function () {
      // It needs some time to generate the proof.
      this.timeout(60000)

      const honkVerifier = await deployHonkVerifier()
      const [signer] = await ethers.getSigners()

      const balance = 100n
      const signature = (await signer.signMessage(SIGNATURE_MESSAGE)) as `0x${string}`
      const address = await getAddressFromSignature(signature, SIGNATURE_MESSAGE_HASH)
      const leaves = [...[10n, 20n, 30n], hashLeaf(address, balance)]

      const proof = await generateMaskVoteProof({
        publicKey,
        merkleLeaves: leaves,
        balance,
        slotAddress: address,
        numOptions: 2,
      })

      const isValid = await honkVerifier.verify(proof.proof, proof.publicInputs)

      expect(isValid).to.be.true
    })

    it('should validate input correctly', async function () {
      // It needs some time to generate the proof.
      this.timeout(60000)

      const mockEnclave = await deployMockEnclave()
      const crispProgram = await deployCRISPProgram({ mockEnclave })
      await mockEnclave.request(await crispProgram.getAddress())
      const [signer] = await ethers.getSigners()

      const e3Id = 0n

      const vote = [10n, 0n]
      const balance = 100n
      const signature = (await signer.signMessage(SIGNATURE_MESSAGE)) as `0x${string}`
      const address = await getAddressFromSignature(signature, SIGNATURE_MESSAGE_HASH)
      const leaves = [...[10n, 20n, 30n], hashLeaf(address, balance)]
      const merkleTree = generateMerkleTree(leaves)

      const proof = await generateVoteProof({
        vote,
        publicKey,
        signature,
        merkleLeaves: leaves,
        balance,
        messageHash: SIGNATURE_MESSAGE_HASH,
        slotAddress: address,
      })

      await mockEnclave.setCommitteePublicKey(proof.publicInputs[1])

      const encodedProof = encodeSolidityProof(proof)

      // Call next functions with fake data for testing.
      await crispProgram.setMerkleRoot(e3Id, merkleTree.root)

      const encodedCustomParams = AbiCoder.defaultAbiCoder().encode(
        ['address', 'uint256', 'uint256', 'uint256', 'uint256'],
        [zeroAddress, 0, 2, 0, 1],
      )
      await crispProgram.validate(e3Id, 0n, '0x', '0x', encodedCustomParams)

      // If it doesn't throw, the test is successful.
      await crispProgram.validateInput(e3Id, zeroAddress, encodedProof)
    })
  })
})
