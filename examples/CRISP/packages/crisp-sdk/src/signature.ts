// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { publicKeyToAddress } from 'viem/utils'
import { SIGNATURE_MESSAGE_HASH } from './constants'
import { hexToBytes, recoverPublicKey } from 'viem'

/**
 * Given a signature, extract the signature components for the Noir signature verification circuit.
 * @param signature The signature to extract the components from.
 * @returns The extracted signature components.
 */
export const extractSignatureComponents = async (
  signature: `0x${string}`,
): Promise<{
  messageHash: Uint8Array
  publicKeyX: Uint8Array
  publicKeyY: Uint8Array
  signature: Uint8Array
}> => {
  const publicKey = await recoverPublicKey({ hash: SIGNATURE_MESSAGE_HASH, signature })
  const publicKeyBytes = hexToBytes(publicKey)
  const publicKeyX = publicKeyBytes.slice(1, 33)
  const publicKeyY = publicKeyBytes.slice(33, 65)

  // Extract r and s from signature (remove v)
  const sigBytes = hexToBytes(signature)
  const r = sigBytes.slice(0, 32) // First 32 bytes
  const s = sigBytes.slice(32, 64) // Next 32 bytes

  const signatureBytes = new Uint8Array(64)
  signatureBytes.set(r, 0)
  signatureBytes.set(s, 32)

  return {
    messageHash: hexToBytes(SIGNATURE_MESSAGE_HASH),
    publicKeyX: publicKeyX,
    publicKeyY: publicKeyY,
    signature: signatureBytes,
  }
}

export const getAddressFromSignature = async (signature: `0x${string}`): Promise<string> => {
  const publicKey = await recoverPublicKey({ hash: SIGNATURE_MESSAGE_HASH, signature })

  return publicKeyToAddress(publicKey)
}
