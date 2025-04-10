import React, { useEffect, useState } from 'react'

interface CardProps {
  children: React.ReactNode
  isDetails?: boolean
  isActive?: boolean
  checked?: boolean
  onChecked?: (clicked: boolean) => void
}

const Card: React.FC<CardProps> = ({ children, isActive, isDetails, checked, onChecked }) => {
  const [isClicked, setIsClicked] = useState<boolean>(checked ?? false)

  useEffect(() => {
    setIsClicked(checked ?? false)
  }, [checked])

  const handleClick = () => {
    if (isDetails) return
    if (onChecked) onChecked(!isClicked)
    setIsClicked(!isClicked)
  }

  useEffect(() => {
    if (isActive) {
      setIsClicked(false)
    }
  }, [isActive])

  return (
    <div
      className={`
        h-full w-full
        cursor-pointer
        ${isDetails ? ' p-4' : 'min-h-[144px] p-10 md:min-h-[288px] md:p-20'}
        rounded-[24px] bg-white text-black
        ${!isDetails && 'shadow-md'}
        transform 
        border-2 transition-all duration-300 ease-in-out 
        ${isClicked ? 'scale-105 border-lime-400' : ''}
        ${isClicked ? 'border-lime-400' : 'border-slate-600/20'}
        ${isClicked ? 'bg-white' : 'bg-slate-100'}
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
