// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import type { NoirSignatureInputs } from './types'

import { hashMessage, hexToBytes, recoverPublicKey } from 'viem'

/**
 * Given a message and its signed version, extract the signature components
 * @param message The original message
 * @param signedMessage The signed message (signature)
 * @returns The extracted signature components
 */
export const extractSignature = async (message: string, signedMessage: `0x${string}`): Promise<NoirSignatureInputs> => {
  const messageHash = hashMessage(message)
  const messageBytes = hexToBytes(messageHash)

  const publicKey = await recoverPublicKey({
    hash: messageHash,
    signature: signedMessage,
  })

  const publicKeyBytes = hexToBytes(publicKey)
  const publicKeyX = publicKeyBytes.slice(1, 33)
  const publicKeyY = publicKeyBytes.slice(33, 65)

  // Extract r and s from signature (remove v)
  const sigBytes = hexToBytes(signedMessage)
  const r = sigBytes.slice(0, 32) // First 32 bytes
  const s = sigBytes.slice(32, 64) // Next 32 bytes

  const signatureBytes = new Uint8Array(64)
  signatureBytes.set(r, 0)
  signatureBytes.set(s, 32)

  return {
    hashed_message: messageBytes,
    pub_key_x: publicKeyX,
    pub_key_y: publicKeyY,
    signature: signatureBytes,
  }
}
