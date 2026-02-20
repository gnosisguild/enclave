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
      className={`
        h-full w-full
        cursor-pointer
        ${isDetails ? ' p-4' : 'min-h-[144px] p-10 md:min-h-[288px] md:p-20'}
        rounded-[24px] bg-white text-black
        ${!isDetails && 'shadow-md'}
        transform 
        border-2 transition-all duration-300 ease-in-out 
        ${derivedIsClicked ? 'scale-105 border-lime-400' : ''}
        ${derivedIsClicked ? 'border-lime-400' : 'border-slate-600/20'}
        ${derivedIsClicked ? 'bg-white' : 'bg-slate-100'}
        ${!isDetails && 'hover:border-lime-300 hover:bg-white hover:shadow-lg'}
        flex w-full items-center justify-center
      `}
      onClick={handleClick}
    >
      {children}
    </div>
  )
}

export default Card
