// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { expect, describe, it, beforeAll } from 'vitest'
import { generateMerkleProof, generateMerkleTree, hashLeaf } from '../src/utils'
import { getTreeData } from '../src'
import { CRISP_SERVER_URL } from './constants'

describe('Utils', () => {
  let leaves: bigint[]

  beforeAll(async () => {
    leaves = await getTreeData(CRISP_SERVER_URL, 0)
  })

  describe('hashLeaf', () => {
    it('should return a bigint hash of the two values', () => {
      const leaf = hashLeaf('0x1234567890123456789012345678901234567890', '1000')
      expect(typeof leaf).toBe('bigint')
    })
  })

  describe('generateMerkleTree', () => {
    it('should generate a merkle tree', () => {
      const tree = generateMerkleTree(leaves)
      expect(tree.root).toBeDefined()
    })
  })

  describe('generateMerkleProof', () => {
    const address = '0x1234567890123456789012345678901234567890'
    const balance = 1000
    it('should generate a merkle proof for a leaf', () => {
      const proof = generateMerkleProof(0, balance, address, leaves)
      expect(proof.leaf).toBe(hashLeaf(address, balance.toString()))
    })
    it('should throw if the leaf does not exist in the tree', () => {
      expect(() => generateMerkleProof(0, balance, address, [])).toThrow('Leaf not found in the tree')
      expect(() => generateMerkleProof(0, 999, address, leaves)).toThrow('Leaf not found in the tree')
    })
  })
})
