// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { describe, it, expect } from 'vitest'

import { extractSignatureComponents } from '../src/signature'
import { SIGNATURE } from './constants'

describe('Signature', () => {
  describe('extractSignature', () => {
    it('should extract signature components correctly', async () => {
      const { hashed_message, pub_key_x, pub_key_y, signature: extractedSignature } = await extractSignatureComponents(SIGNATURE)

      expect(hashed_message).toBeInstanceOf(Uint8Array)
      expect(pub_key_x).toBeInstanceOf(Uint8Array)
      expect(pub_key_y).toBeInstanceOf(Uint8Array)
      expect(extractedSignature).toBeInstanceOf(Uint8Array)
    })
  })
})
