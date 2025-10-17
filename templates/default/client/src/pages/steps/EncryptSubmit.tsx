// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { useState, useEffect } from 'react'
import { LockIcon, CheckCircleIcon } from '@phosphor-icons/react'
import CardContent from '../components/CardContent'
import Spinner from '../components/Spinner'
import ErrorDisplay from '../components/ErrorDisplay'
import { useWizard, WizardStep } from '../../context/WizardContext'
import { decodePlaintextOutput } from '@enclave-e3/sdk'

/**
 * EncryptSubmit component - Fifth step in the Enclave wizard flow
 *
 * This component handles the encryption and submission of user inputs to the E3.
 * It provides feedback on the encryption process and displays the status of the
 * submission to the E3.
 */
const EncryptSubmit: React.FC = () => {
  const { e3State, setE3State, setResult, setCurrentStep, inputPublishError, inputPublishSuccess, handleTryAgain, sdk } = useWizard()
  const { isInitialized, onEnclaveEvent, off, EnclaveEventType } = sdk

  const [showErrorDetails, setShowErrorDetails] = useState(false)

  // Set up event listeners for this step
  useEffect(() => {
    if (!isInitialized) return

    const handlePlaintextOutput = (event: any) => {
      const { e3Id, plaintextOutput } = event.data
      setE3State((prev) => {
        if (prev.id !== null && e3Id === prev.id) {
          const decodedResult = decodePlaintextOutput(plaintextOutput)
          setResult(decodedResult)
          return {
            ...prev,
            plaintextOutput: plaintextOutput as string,
            hasPlaintextOutput: true,
          }
        }
        return prev
      })
    }

    onEnclaveEvent(EnclaveEventType.PLAINTEXT_OUTPUT_PUBLISHED, handlePlaintextOutput)

    return () => {
      off(EnclaveEventType.PLAINTEXT_OUTPUT_PUBLISHED, handlePlaintextOutput)
    }
  }, [isInitialized, onEnclaveEvent, off, EnclaveEventType, setE3State, setResult])

  // Auto-advance to results when output is available
  useEffect(() => {
    if (e3State.hasPlaintextOutput) {
      setCurrentStep(WizardStep.RESULTS)
    }
  }, [e3State.hasPlaintextOutput, setCurrentStep])

  return (
    <CardContent>
      <div className='space-y-6 text-center'>
        <div className='flex justify-center'>
          <LockIcon size={48} className='text-enclave-400' />
        </div>
        <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 5: Encrypting & Submitting</p>
        <div className='space-y-4'>
          <h3 className='text-lg font-semibold text-slate-700'>Secure Process Execution</h3>

          {!inputPublishError && !inputPublishSuccess && (
            <div className='space-y-4'>
              <div className='flex justify-center'>
                <Spinner size={40} />
              </div>
              <p className='text-slate-600'>
                Your inputs are being encrypted to the committee's public key and submitted to the E3. The Compute Provider will execute the
                FHE computation over your encrypted data...
              </p>
              <div className='rounded-lg border border-blue-200 bg-blue-50 p-4'>
                <p className='text-sm text-slate-600'>
                  <strong>Process:</strong> Encrypt to Key → Submit to E3 → FHE Computation → Ciphertext Output
                </p>
              </div>
            </div>
          )}

          {inputPublishError && (
            <div className='space-y-4'>
              <ErrorDisplay
                error={inputPublishError}
                showDetails={showErrorDetails}
                onToggleDetails={() => setShowErrorDetails(!showErrorDetails)}
              />
              <button
                onClick={handleTryAgain}
                className='w-full rounded-lg bg-red-500 px-6 py-3 font-semibold text-white transition-all duration-200 hover:bg-red-600'
              >
                Try Again
              </button>
            </div>
          )}

          {inputPublishSuccess && (
            <div className='space-y-4'>
              <div className='flex justify-center'>
                <CheckCircleIcon size={48} className='text-green-500' />
              </div>
              <div className='rounded-lg border border-green-200 bg-green-50 p-4'>
                <p className='text-sm text-green-600'>
                  <strong>✅ Inputs Successfully Submitted!</strong>
                  <br />
                  Your encrypted inputs have been published to the E3. The Compute Provider is executing the FHE computation and will
                  publish the ciphertext output for committee decryption.
                </p>
              </div>
              <div className='rounded-lg border border-yellow-200 bg-yellow-50 p-4'>
                <div className='mb-2 flex justify-center'>
                  <Spinner size={20} />
                </div>
                <p className='text-sm text-slate-600'>
                  <strong>Computing...</strong> Waiting for the Ciphernode Committee to collectively decrypt the verified output.
                </p>
              </div>
            </div>
          )}
        </div>
      </div>
    </CardContent>
  )
}

export default EncryptSubmit
