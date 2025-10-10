// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React from 'react'
import { CheckCircleIcon } from '@phosphor-icons/react'
import CardContent from '../components/CardContent'
import { useWizard } from '../../context/WizardContext'

/**
 * Results component - Sixth step in the Enclave wizard flow
 *
 * This component displays the results of the computation, including the encrypted
 * computation, the E3 ID, the transaction hash, and the raw output.
 */
const Results: React.FC = () => {
  const { submittedInputs, result, e3State, lastTransactionHash, handleReset } = useWizard()

  const onReset = () => {
    handleReset()
  }

  return (
    <CardContent>
      <div className='space-y-6 text-center'>
        <div className='flex justify-center'>
          <CheckCircleIcon size={48} className='text-green-500' />
        </div>
        <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 6: Results</p>
        <div className='space-y-4'>
          <h3 className='text-lg font-semibold text-slate-700'>Computation Complete!</h3>

          <div className='rounded-lg border border-green-200 bg-green-50 p-6'>
            <div className='space-y-3'>
              <p className='text-lg font-semibold text-slate-700'>
                <strong>Your Encrypted Computation:</strong>
              </p>
              <p className='text-2xl font-bold text-green-700'>
                {submittedInputs
                  ? `${submittedInputs.input1} + ${submittedInputs.input2} = ${result !== null ? result : 'Computing...'}`
                  : 'Computing...'}
              </p>
              {result !== null && <p className='text-sm text-slate-600'>✅ Computed securely using FHE with distributed key decryption!</p>}
            </div>
          </div>

          <div className='grid grid-cols-1 gap-3 text-left'>
            <div className='rounded-lg border border-slate-200 bg-slate-50 p-4'>
              <p className='text-sm text-slate-600'>
                <strong>E3 ID:</strong> {String(e3State.id)}
              </p>
            </div>
            {lastTransactionHash && (
              <div className='rounded-lg border border-slate-200 bg-slate-50 p-4'>
                <p className='text-sm text-slate-600'>
                  <strong>Transaction:</strong> {lastTransactionHash.slice(0, 10)}...{lastTransactionHash.slice(-8)}
                </p>
              </div>
            )}
            {e3State.plaintextOutput && (
              <div className='rounded-lg border border-slate-200 bg-slate-50 p-4'>
                <p className='text-sm text-slate-600'>
                  <strong>Raw Output:</strong> {e3State.plaintextOutput.slice(0, 20)}...
                </p>
              </div>
            )}
          </div>

          <div className='rounded-lg border border-blue-200 bg-blue-50 p-4'>
            <p className='text-sm text-slate-600'>
              <strong>🔒 Cryptographic Guarantees:</strong> Your inputs remained encrypted throughout the entire process. The Ciphernode
              Committee used distributed key cryptography to decrypt only the verified output, ensuring data privacy, data integrity, and
              correct execution.
            </p>
          </div>
        </div>

        <button
          onClick={onReset}
          className='w-full rounded-lg bg-enclave-400 px-6 py-3 font-semibold text-slate-800 transition-all duration-200 hover:bg-enclave-300 hover:shadow-md'
        >
          Start New Computation
        </button>
      </div>
    </CardContent>
  )
}

export default Results
