// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { useState } from 'react'
import { NumberSquareOneIcon } from '@phosphor-icons/react'
import { hexToBytes } from 'viem'
import CardContent from '../components/CardContent'
import { useWizard, WizardStep } from '../../context/WizardContext'

/**
 * EnterInputs component - Fourth step in the Enclave wizard flow
 *
 * This component handles the input of two numbers for a privacy-preserving addition
 * using fully homomorphic encryption (FHE). It provides feedback on the input process
 * and displays the status of the input submission.
 */
const EnterInputs: React.FC = () => {
  const [input1, setInput1] = useState('')
  const [input2, setInput2] = useState('')
  const { e3State, setCurrentStep, setLastTransactionHash, setInputPublishError, setInputPublishSuccess, setSubmittedInputs, sdk } =
    useWizard()
  const { publishInput } = sdk

  const handleInputSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    console.log('handleInputSubmit')
    if (!input1 || !input2 || e3State.publicKey === null || e3State.id === null) {
      console.log('Refusing to submit input because input is empty or publickey is null or is is null')
      return
    }

    setCurrentStep(WizardStep.ENCRYPT_SUBMIT)
    setInputPublishError(null)
    setInputPublishSuccess(false)

    try {
      // Store the inputs in context for the Results component
      setSubmittedInputs({ input1, input2 })

      // Parse inputs
      const num1 = BigInt(input1)
      const num2 = BigInt(input2)

      // Convert hex public key to bytes
      const publicKeyBytes = hexToBytes(e3State.publicKey)

      // Encrypt both inputs
      const encryptedInput1 = await sdk.sdk?.encryptNumber(num1, publicKeyBytes)
      const encryptedInput2 = await sdk.sdk?.encryptNumber(num2, publicKeyBytes)

      if (!encryptedInput1 || !encryptedInput2) {
        throw new Error('Failed to encrypt inputs')
      }

      // Publish first input
      await publishInput(e3State.id, `0x${Array.from(encryptedInput1, (b) => b.toString(16).padStart(2, '0')).join('')}` as `0x${string}`)

      // Publish second input
      const hash2 = await publishInput(
        e3State.id,
        `0x${Array.from(encryptedInput2, (b: any) => b.toString(16).padStart(2, '0')).join('')}` as `0x${string}`,
      )

      setLastTransactionHash(hash2)
      setInputPublishSuccess(true)
    } catch (error) {
      setInputPublishError(error instanceof Error ? error.message : 'Failed to encrypt and publish inputs')
      console.error('Error encrypting/publishing inputs:', error)
    }
  }

  return (
    <CardContent>
      <form onSubmit={handleInputSubmit} className='space-y-6 text-center'>
        <div className='flex justify-center'>
          <NumberSquareOneIcon size={48} className='text-enclave-400' />
        </div>
        <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 4: Enter Your Numbers</p>
        <div className='space-y-4'>
          <h3 className='text-lg font-semibold text-slate-700'>Homomorphic Encrypted Computation</h3>
          <p className='leading-relaxed text-slate-600'>
            Enter two numbers for a privacy-preserving addition using fully homomorphic encryption (FHE). Your inputs will be encrypted
            locally and remain encrypted throughout the entire computation process, with only the final result being decrypted.
          </p>
          <div className='rounded-lg border border-blue-200 bg-blue-50 p-4'>
            <p className='text-sm text-slate-600'>
              <strong>Privacy Guarantee:</strong> FHE allows computation on encrypted data. Your numbers remain private throughout the
              process - inputs, intermediate states, and execution are all encrypted.
            </p>
          </div>

          <div className='space-y-4'>
            <div>
              <label htmlFor='input1' className='mb-2 block text-sm font-medium text-slate-700'>
                First Number
              </label>
              <input
                id='input1'
                type='number'
                value={input1}
                onChange={(e) => setInput1(e.target.value)}
                className='w-full rounded-md border border-slate-300 px-3 py-2 focus:border-transparent focus:outline-none focus:ring-2 focus:ring-enclave-500'
                placeholder='Enter first number'
                required
              />
            </div>
            <div>
              <label htmlFor='input2' className='mb-2 block text-sm font-medium text-slate-700'>
                Second Number
              </label>
              <input
                id='input2'
                type='number'
                value={input2}
                onChange={(e) => setInput2(e.target.value)}
                className='w-full rounded-md border border-slate-300 px-3 py-2 focus:border-transparent focus:outline-none focus:ring-2 focus:ring-enclave-500'
                placeholder='Enter second number'
                required
              />
            </div>
          </div>

          {input1 && input2 && (
            <div className='rounded-lg border border-enclave-200 bg-enclave-50 p-4'>
              <p className='text-sm text-slate-600'>
                <strong>Ready to compute:</strong> {input1} + {input2} = ?
              </p>
            </div>
          )}
        </div>

        <button
          type='submit'
          disabled={!input1 || !input2 || !e3State.isActivated}
          className='w-full rounded-lg bg-enclave-400 px-6 py-3 font-semibold text-slate-800 transition-all duration-200 hover:bg-enclave-300 hover:shadow-md disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-500'
        >
          {!e3State.isActivated ? 'E3 Not Activated Yet' : !input1 || !input2 ? 'Enter Both Numbers' : 'Proceed to Encryption'}
        </button>
      </form>
    </CardContent>
  )
}

export default EnterInputs
