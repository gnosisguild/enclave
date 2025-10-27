// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { describe, it, expect } from 'vitest'

import { extractSignature } from '../src/signature'

describe('Signature', () => {
  describe('extractSignature', () => {
    it('should extract signature components correctly', async () => {
      const message = 'Vote for round 0'
      const signature =
        '0x1641431d0ed3fd86814f026da62e11434b53c6a85162fea7f99218bf3c307dec7f361c235f07b658780afd91a5c9d68d6a4b14d5eb0511f6688d3e91140eec121b'

      const { hashed_message, pub_key_x, pub_key_y, signature: extractedSignature } = await extractSignature(message, signature)

      expect(hashed_message).toBeInstanceOf(Uint8Array)
      expect(pub_key_x).toBeInstanceOf(Uint8Array)
      expect(pub_key_y).toBeInstanceOf(Uint8Array)
      expect(extractedSignature).toBeInstanceOf(Uint8Array)
    })
  })
})
