// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React from 'react'

interface CardContentProps {
  children: React.ReactNode
}

const CardContent: React.FC<CardContentProps> = ({ children }) => {
  return (
    <div className='z-50 w-full max-w-screen-md space-y-10 rounded-2xl border border-slate-300 bg-white p-8 shadow-[0_0_20px_rgba(0,0,0,0.1)] md:p-12'>
      {children}
    </div>
  )
}

export default CardContent
