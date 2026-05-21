// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { useMemo, useState } from 'react'

interface CardProps {
  children: React.ReactNode
  isDetails?: boolean
  isActive?: boolean
  checked?: boolean
  onChecked?: (clicked: boolean) => void
}

const Card: React.FC<CardProps> = ({ children, isActive, isDetails, checked, onChecked }) => {
  const [isClicked, setIsClicked] = useState<boolean>(checked ?? false)

  const derivedIsClicked = useMemo(() => {
    if (isActive) return false
    return checked ?? isClicked
  }, [isActive, checked, isClicked])

  const handleClick = () => {
    if (isDetails) return
    if (onChecked) onChecked(!derivedIsClicked)
    setIsClicked(!derivedIsClicked)
  }

  return (
    <div
      data-test-id='card'
      className={`faceoff-slot ${derivedIsClicked ? 'selected' : ''}`}
      style={{ aspectRatio: '1 / 1', cursor: isDetails ? 'default' : 'pointer', minHeight: isDetails ? 96 : 144 }}
      onClick={handleClick}
    >
      {children}
    </div>
  )
}

export default Card
