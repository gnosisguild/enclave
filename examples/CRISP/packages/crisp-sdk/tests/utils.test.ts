// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { expect, describe, it } from 'vitest'
import { generateMerkleProof, generateMerkleTree, hashLeaf } from '../src/utils'
import { LEAVES, MAX_DEPTH } from './constants'

describe('Utils', () => {
  describe('hashLeaf', () => {
    it('should return a bigint hash of the two values', () => {
      const leaf = hashLeaf('0x1234567890123456789012345678901234567890', '1000')
      expect(typeof leaf).toBe('bigint')
    })
  })

  describe('generateMerkleTree', () => {
    it('should generate a merkle tree', () => {
      const tree = generateMerkleTree(LEAVES)
      expect(tree.root).toBeDefined()
    })
  })

  describe('generateMerkleProof', () => {
    const address = '0x1234567890123456789012345678901234567890'
    const balance = 1000n
    it('should generate a merkle proof for a leaf', () => {
      const proof = generateMerkleProof(0, balance, address, LEAVES, MAX_DEPTH)
      expect(proof.leaf).toBe(hashLeaf(address, balance.toString()))

      expect(proof.length).toBe(4)
      expect(proof.indices.length).toBe(MAX_DEPTH)
    })
    it('should throw if the leaf does not exist in the tree', () => {
      expect(() => generateMerkleProof(0, balance, address, [], MAX_DEPTH)).toThrow('Leaf not found in the tree')
      expect(() => generateMerkleProof(0, 999n, address, LEAVES, MAX_DEPTH)).toThrow('Leaf not found in the tree')
    })
  })
})
