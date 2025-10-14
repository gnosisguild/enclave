// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React from 'react'

interface SDKErrorDisplayProps {
  error: string
}

const SDKErrorDisplay: React.FC<SDKErrorDisplayProps> = ({ error }) => (
  <div className='min-h-screen bg-gray-100 px-4 py-12 sm:px-6 lg:px-8'>
    <div className='mx-auto max-w-md'>
      <div className='rounded-md border border-red-200 bg-red-50 p-4'>
        <div className='flex'>
          <div className='ml-3'>
            <h3 className='text-sm font-medium text-red-800'>SDK Error</h3>
            <div className='mt-2 text-sm text-red-700'>{error}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
)

export default SDKErrorDisplay
