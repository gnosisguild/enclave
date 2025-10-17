// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React from 'react'
import { WarningIcon } from '@phosphor-icons/react'

interface EnvironmentErrorProps {
  missingVars: string[]
}

const EnvironmentError: React.FC<EnvironmentErrorProps> = ({ missingVars }) => {
  return (
    <div className='flex min-h-screen items-center justify-center bg-gradient-to-br from-slate-50 to-slate-100 p-4'>
      <div className='w-full max-w-2xl rounded-2xl border border-red-200 bg-white p-8 shadow-xl'>
        <div className='text-center'>
          <div className='mx-auto mb-6 flex h-16 w-16 items-center justify-center rounded-full bg-red-100'>
            <WarningIcon size={32} className='text-red-600' />
          </div>

          <h1 className='mb-4 text-2xl font-bold text-gray-900'>Environment Configuration Required</h1>

          <p className='mb-6 text-gray-600'>
            The following environment variables need to be configured before you can use the encrypted computation features:
          </p>

          <div className='mb-6 rounded-lg bg-gray-50 p-4'>
            <ul className='space-y-2 text-left'>
              {missingVars.map((varName) => (
                <li key={varName} className='flex items-center space-x-2'>
                  <code className='rounded bg-red-100 px-2 py-1 font-mono text-sm text-red-700'>{varName}</code>
                </li>
              ))}
            </ul>
          </div>

          <div className='rounded-lg border border-blue-200 bg-blue-50 p-4 text-left'>
            <h3 className='mb-2 font-semibold text-blue-900'>How to configure:</h3>
            <ol className='list-inside list-decimal space-y-1 text-sm text-blue-800'>
              <li>
                Create a <code className='rounded bg-blue-100 px-1'>.env</code> file in the client directory
              </li>
              <li>Add the missing environment variables with their appropriate values</li>
              <li>Restart the development server</li>
            </ol>
          </div>

          <div className='mt-6'>
            <button
              onClick={() => window.location.reload()}
              className='w-full rounded-lg bg-blue-600 px-6 py-3 font-semibold text-white transition-all duration-200 hover:bg-blue-500 hover:shadow-md disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-500'
            >
              Reload Page
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

export default EnvironmentError
