// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React from 'react'
import { formatContractError } from '@/utils/error-formatting'

interface ErrorDisplayProps {
  error: any
  showDetails: boolean
  onToggleDetails: () => void
}

const ErrorDisplay: React.FC<ErrorDisplayProps> = ({ error, showDetails, onToggleDetails }) => {
  if (!error) return null

  const userMessage = formatContractError(error)
  const technicalMessage = error.message || JSON.stringify(error, null, 2)

  return (
    <div className='rounded-lg border border-red-200 bg-red-50 p-4'>
      <p className='mb-2 text-sm text-red-600'>
        <strong>Error:</strong> {userMessage}
      </p>
      {userMessage !== technicalMessage && (
        <button onClick={onToggleDetails} className='text-xs text-red-500 underline hover:text-red-700'>
          {showDetails ? 'Hide Details' : 'Show Technical Details'}
        </button>
      )}
      {showDetails && userMessage !== technicalMessage && (
        <pre className='mt-2 overflow-x-auto rounded border border-red-300 bg-red-100 p-2 text-xs text-red-800'>{technicalMessage}</pre>
      )}
    </div>
  )
}

export default ErrorDisplay
