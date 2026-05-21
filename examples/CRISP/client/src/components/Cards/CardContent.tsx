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
    <div className='card col' style={{ width: '100%', maxWidth: 720, gap: 28, padding: 32 }}>
      {children}
    </div>
  )
}

export default CardContent
