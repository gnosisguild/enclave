// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { concat, getBytes, keccak256, toUtf8Bytes, Wallet } from "ethers";

import { describe, it } from "node:test";

describe("Governance Circuit", () => {
    const privateKey = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
    const wallet = new Wallet(privateKey)

    describe("Signature Verification", () => {
        it("should print the inputs for the ecdsa circuit", async() => {
            const message = "Hello, Enclave!"
            const signature = await wallet.signMessage(message)
      
            // Get the public key from the wallet (uncompressed, 65 bytes: 0x04 + x + y)
            const publicKey = wallet.signingKey.publicKey
            const publicKeyBytes = getBytes(publicKey)
      
            // Extract x and y coordinates (skip first byte 0x04)
            const pub_key_x = Array.from(publicKeyBytes.slice(1, 33))
            const pub_key_y = Array.from(publicKeyBytes.slice(33, 65))
      
            // Split signature into r and s (each 32 bytes)
            const sigBytes = getBytes(signature)
            const r = Array.from(sigBytes.slice(0, 32))
            const s = Array.from(sigBytes.slice(32, 64))
            const sig = [...r, ...s] // 64 bytes total
      
            // Hash the message with Ethereum prefix (same as signMessage does)
            const messageBytes = toUtf8Bytes(message)
            const messagePrefix = toUtf8Bytes(`\x19Ethereum Signed Message:\n${messageBytes.length}`)
            const prefixedMessage = concat([messagePrefix, messageBytes])
            const hashed_message = Array.from(getBytes(keccak256(prefixedMessage)))
      
            // Prepare inputs for the circuit
            const inputs = {
              hashed_message,
              pub_key_x,
              pub_key_y,
              signature: sig
            }
      
            console.log("Inputs:", inputs)
            console.log("Wallet address:", wallet.address)
        })
    })

    describe("Address Derivation", () => {
        it("should print the inputs for the address derivation circuit", async() => {
            const publicKey = wallet.signingKey.publicKey
            const publicKeyBytes = getBytes(publicKey)
      
            const pub_key_x = Array.from(publicKeyBytes.slice(1, 33))
            const pub_key_y = Array.from(publicKeyBytes.slice(33, 65))
      
            // You can test this with your derive_address circuit function
            console.log("pub_key_x:", pub_key_x)
            console.log("pub_key_y:", pub_key_y)
            const addressBytes = Array.from(getBytes(wallet.address)) // This gives you [u8;20]

            console.log("Address as byte array:", addressBytes)
        })
    })
})