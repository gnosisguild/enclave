// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { describe, expect, it } from 'vitest'
import { CompiledCircuit } from '@noir-lang/noir_js'

import { EnclaveSDK } from '../src/enclave-sdk'
import { zeroAddress } from 'viem'
import demoCircuit from './fixtures/demo_circuit.json'

describe('encryptNumber', () => {
  describe('trbfv', () => {
    // create SDK with default config
    const sdk = EnclaveSDK.create({
      chainId: 31337,
      contracts: {
        enclave: zeroAddress,
        ciphernodeRegistry: zeroAddress,
        feeToken: zeroAddress,
      },
      rpcUrl: '',
      privateKey: '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80',
      thresholdBfvParamsPresetName: 'INSECURE_THRESHOLD_512',
    })

    it('should encrypt a number without crashing in a node environent', async () => {
      const publicKey = await sdk.generatePublicKey()
      const value = await sdk.encryptNumber(10n, publicKey)
      expect(value).to.be.an.instanceof(Uint8Array)
      expect(value.length).to.equal(9_242)
      // TODO: test the encryption is correct
    })
    it('should encrypt a number and generate a proof without crashing in a node environent', async () => {
      const publicKey = await sdk.generatePublicKey()

      const value = await sdk.encryptNumberAndGenProof(1n, publicKey, demoCircuit as unknown as CompiledCircuit)

      expect(value).to.be.an.instanceof(Object)
      expect(value.encryptedData).to.be.an.instanceof(Uint8Array)
      expect(value.proof).to.be.an.instanceOf(Object)
    }, 9999999)

    it('should encrypt a vector of numbers without crashing in a node environent', async () => {
      const publicKey = await sdk.generatePublicKey()
      const value = await sdk.encryptVector(new BigUint64Array([1n, 2n]), publicKey)
      expect(value).to.be.an.instanceof(Uint8Array)
      expect(value.length).to.equal(9_242)
    })

    it('should encrypt a vector and generate a proof without crashing in a node environent', async () => {
      const publicKey = await sdk.generatePublicKey()

      const value = await sdk.encryptVectorAndGenProof(new BigUint64Array([1n, 2n]), publicKey, demoCircuit as unknown as CompiledCircuit)

      expect(value).to.be.an.instanceof(Object)
      expect(value.encryptedData).to.be.an.instanceof(Uint8Array)
      expect(value.proof).to.be.an.instanceOf(Object)
    }, 9999999)
  })
})
