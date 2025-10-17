// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { useState, useEffect } from 'react'
import { LockIcon } from '@phosphor-icons/react'
import CardContent from '../components/CardContent'
import Spinner from '../components/Spinner'
import ErrorDisplay from '../components/ErrorDisplay'
import { useWizard, WizardStep } from '../../context/WizardContext'

/**
 * ActivateE3 component - Third step in the Enclave wizard flow
 *
 * This component handles the activation of the E3 using the Ciphernode Committee's
 * shared public key. It provides feedback on the activation process and displays
 * the status of the activation.
 */
const ActivateE3: React.FC = () => {
  const { e3State, setE3State, setLastTransactionHash, setCurrentStep, sdk } = useWizard()
  const { isInitialized, activateE3, onEnclaveEvent, off, EnclaveEventType } = sdk

  const [isRequesting, setIsRequesting] = useState(false)
  const [requestError, setRequestError] = useState<any>(null)
  const [requestSuccess, setRequestSuccess] = useState(false)
  const [lastTransactionHash, setLocalTransactionHash] = useState<string | undefined>()
  const [showErrorDetails, setShowErrorDetails] = useState(false)

  // Set up event listeners for this step
  useEffect(() => {
    if (!isInitialized) return

    const handleE3Activated = (event: any) => {
      const { e3Id, expiration } = event.data
      setE3State((prev) => {
        if (prev.id !== null && e3Id === prev.id) {
          return {
            ...prev,
            isActivated: true,
            expiresAt: expiration || null,
          }
        }
        return prev
      })
    }

    onEnclaveEvent(EnclaveEventType.E3_ACTIVATED, handleE3Activated)

    return () => {
      off(EnclaveEventType.E3_ACTIVATED, handleE3Activated)
    }
  }, [isInitialized, onEnclaveEvent, off, EnclaveEventType, setE3State])

  // Auto-advance to next step when E3 is activated
  useEffect(() => {
    if (e3State.isActivated) {
      setCurrentStep(WizardStep.ENTER_INPUTS)
    }
  }, [e3State.isActivated, setCurrentStep])

  const handleActivateE3 = async () => {
    console.log('handleActivateE3')

    if (e3State.id === null || e3State.publicKey === null) {
      console.log('refusing to run handler because id or publicKey is null')
      return
    }
    setIsRequesting(true)
    setRequestError(null)

    try {
      const hash = await activateE3(e3State.id, e3State.publicKey)
      setLocalTransactionHash(hash)
      setLastTransactionHash(hash)
      setRequestSuccess(true)
    } catch (error) {
      setRequestError(error)
      console.error('Error activating E3:', error)
    } finally {
      setIsRequesting(false)
    }
  }

  return (
    <CardContent>
      <div className='space-y-6 text-center'>
        <div className='flex justify-center'>
          <LockIcon size={48} className='text-enclave-400' />
        </div>
        <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 3: Activate E3</p>
        <div className='space-y-4'>
          <h3 className='text-lg font-semibold text-slate-700'>Activate Encrypted Execution Environment</h3>
          <p className='leading-relaxed text-slate-600'>
            Activate the E3 using the Ciphernode Committee's shared public key. This distributed key ensures no single node can decrypt your
            inputs or intermediate states - only the verified final output can be collectively decrypted by the committee.
          </p>

          {e3State.isActivated && e3State.expiresAt && (
            <div className='rounded-lg border border-enclave-200 bg-enclave-50 p-4'>
              <p className='text-sm text-slate-600'>
                <strong>✅ E3 Environment Activated!</strong>
                <br />
                <strong>Expires At:</strong> {new Date(Number(e3State.expiresAt) * 1000).toLocaleString()}
              </p>
            </div>
          )}

          {requestError && (
            <ErrorDisplay
              error={requestError}
              showDetails={showErrorDetails}
              onToggleDetails={() => setShowErrorDetails(!showErrorDetails)}
            />
          )}

          {requestSuccess && lastTransactionHash && (
            <div className='rounded-lg border border-green-200 bg-green-50 p-4'>
              <p className='text-sm text-green-600'>
                <strong>✅ Transaction Successful!</strong>
                <br />
                Hash: {lastTransactionHash.slice(0, 10)}...{lastTransactionHash.slice(-8)}
              </p>
            </div>
          )}
        </div>

        {isRequesting && (
          <div className='mb-4 flex justify-center'>
            <Spinner />
          </div>
        )}

        <button
          onClick={handleActivateE3}
          disabled={isRequesting || e3State.isActivated || !e3State.publicKey}
          className='w-full rounded-lg bg-enclave-400 px-6 py-3 font-semibold text-slate-800 transition-all duration-200 hover:bg-enclave-300 hover:shadow-md disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-500'
        >
          {isRequesting
            ? 'Activating...'
            : e3State.isActivated
              ? 'E3 Activated - Ready for Input!'
              : !e3State.publicKey
                ? 'Waiting for Committee Key...'
                : 'Activate E3 Environment'}
        </button>
      </div>
    </CardContent>
  )
}

export default ActivateE3
