// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { expect, describe, it } from 'vitest'
import { extractSignatureComponents, generateMerkleProof, generateMerkleTree, hashLeaf } from '../src/utils'
import { LEAVES } from './constants'
import { MASK_SIGNATURE } from '../src/constants'

describe('Utils', () => {
  describe('hashLeaf', () => {
    it('Should return a bigint hash of the two values', () => {
      const leaf = hashLeaf('0x1234567890123456789012345678901234567890', 1000n)

      expect(typeof leaf).toBe('bigint')
      expect(leaf).toBe(5744770974032406598001112375731623179326875761382288642755141437508907349272n)
    })
  })

  describe('generateMerkleTree', () => {
    it('Should generate a merkle tree', () => {
      const tree = generateMerkleTree(LEAVES)

      expect(tree.root).toBeDefined()
    })
  })

  describe('generateMerkleProof', () => {
    const address = '0x145B2260E2DAa2965F933A76f5ff5aE3be5A7e5a'
    const balance = 100n

    it('Should generate a valid merkle proof for a leaf', () => {
      const tree = generateMerkleTree(LEAVES)

      const proof = generateMerkleProof(balance, address, LEAVES)
      expect(proof.leaf).toBe(hashLeaf(address, balance))

      expect(proof.length).toBe(3)
      // Unpad the proof for verification
      const unpaddedProof = {
        ...proof.proof,
        siblings: proof.proof.siblings.slice(0, proof.length),
      }

      expect(tree.verifyProof(unpaddedProof)).toBe(true)
    })

    it('Should throw if the leaf does not exist in the tree', () => {
      expect(() => generateMerkleProof(balance, address, [])).toThrow('Leaf not found in the tree')
      expect(() => generateMerkleProof(999n, address, LEAVES)).toThrow('Leaf not found in the tree')
    })
  })

  describe('extractSignatureComponents', () => {
    it('Should extract signature components correctly', async () => {
      const { messageHash, publicKeyX, publicKeyY, signature: extractedSignature } = await extractSignatureComponents(MASK_SIGNATURE)

      expect(messageHash).toBeInstanceOf(Uint8Array)
      expect(publicKeyX).toBeInstanceOf(Uint8Array)
      expect(publicKeyY).toBeInstanceOf(Uint8Array)
      expect(extractedSignature).toBeInstanceOf(Uint8Array)
    })
  })
})
