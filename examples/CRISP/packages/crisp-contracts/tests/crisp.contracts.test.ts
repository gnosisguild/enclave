// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { zeroAddress } from 'viem'
import { ZKInputsGenerator } from '@crisp-e3/zk-inputs'
import {
  encryptVoteAndGenerateCRISPInputs,
  generateProof,
  VotingMode,
  encodeVote,
  MESSAGE,
  generateMerkleProof,
  hashLeaf,
  encodeSolidityProof,
} from '@crisp-e3/sdk'
import { expect } from 'chai'
import { deployCRISPProgram, deployHonkVerifier, deployMockEnclave, nonZeroAddress, ethers } from './utils'

let zkInputsGenerator = ZKInputsGenerator.withDefaults()
let publicKey = zkInputsGenerator.generatePublicKey()
const previousCiphertext = zkInputsGenerator.encryptVote(publicKey, new BigInt64Array([0n]))

describe('CRISP Contracts', function () {
  describe('decode tally', () => {
    it('should decode different tallies correctly', async () => {
      const mockEnclave = await deployMockEnclave()
      const crispProgram = await deployCRISPProgram({ mockEnclave })

      // 2 * 2 + 1 * 1 = 5 Y
      // 2 * 1 + 0 * 1 = 2 N
      const tally1 = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
      ]

      await mockEnclave.setPlaintextOutput(tally1)

      const decodedTally1 = await crispProgram.decodeTally(0)

      expect(decodedTally1[0]).to.equal(5n)
      expect(decodedTally1[1]).to.equal(2n)

      // 1 * 1 + 2 * 2 + 5 * 16 + 8 * 1024 = 8277
      // 2 * 1 + 3 * 64 + 1024 =
      const tally2 = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 5, 0, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 1, 0, 0, 0, 3, 0, 0, 0, 0, 1, 0,
      ]
      await mockEnclave.setPlaintextOutput(tally2)

      const decodedTally2 = await crispProgram.decodeTally(0)

      expect(decodedTally2[0]).to.equal(8277n)
      expect(decodedTally2[1]).to.equal(1218n)
    })
  })

  describe('validate input', () => {
    it('should verify the proof correctly with the crisp verifier', async function () {
      // It needs some time to generate the proof.
      this.timeout(60000)

      const [signer] = await ethers.getSigners()

      const honkVerifier = await deployHonkVerifier()

      const slotAddress = signer.address.toLowerCase() as `0x${string}`

      const vote = { yes: 10n, no: 0n }
      const balance = vote.yes
      const encodedVote = encodeVote(vote, VotingMode.GOVERNANCE, balance)

      const signature = (await signer.signMessage(MESSAGE)) as `0x${string}`
      const leaf = hashLeaf(slotAddress, balance.toString())
      const leaves = [...[10n, 20n], leaf]

      const threshold = 0n
      const merkleProof = generateMerkleProof(threshold, balance, slotAddress, leaves)

      const inputs = await encryptVoteAndGenerateCRISPInputs({
        encodedVote,
        publicKey,
        previousCiphertext,
        signature,
        message: MESSAGE,
        merkleData: merkleProof,
        balance,
        slotAddress,
        isFirstVote: true,
      })

      const proof = await generateProof(inputs)

      const isValid = await honkVerifier.verify(proof.proof, proof.publicInputs)

      expect(isValid).to.be.true
    })

    it.only('should validate input correctly', async function () {
      // It needs some time to generate the proof.
      this.timeout(60000)

      const [signer] = await ethers.getSigners()

      const crispProgram = await deployCRISPProgram()

      const address = signer.address.toLowerCase() as `0x${string}`

      const e3Id = 1n

      const vote = { yes: 10n, no: 0n }
      const balance = vote.yes
      const encodedVote = encodeVote(vote, VotingMode.GOVERNANCE, balance)

      const signature = (await signer.signMessage(MESSAGE)) as `0x${string}`
      const leaf = hashLeaf(address, balance.toString())
      const leaves = [...[10n, 20n], leaf]

      const threshold = 0n
      const merkleProof = generateMerkleProof(threshold, balance, address, leaves)

      const inputs = await encryptVoteAndGenerateCRISPInputs({
        encodedVote,
        publicKey,
        previousCiphertext,
        signature,
        message: MESSAGE,
        merkleData: merkleProof,
        balance,
        slotAddress: address,
        isFirstVote: true,
      })

      const proof = await generateProof(inputs)
      const encodedProof = encodeSolidityProof(proof)

      // Call next functions with fake data for testing.
      await crispProgram.setRoundData(e3Id, merkleProof.proof.root, nonZeroAddress, 1n)
      await crispProgram.validate(e3Id, 0n, '0x', '0x')

      // If it doesn't throw, the test is successful.
      await crispProgram.validateInput(e3Id, zeroAddress, encodedProof)
    })
  })
})
