import React from 'react'

interface CardContentProps {
  children: React.ReactNode
}

const CardContent: React.FC<CardContentProps> = ({ children }) => {
  return (
    <div className='z-50 w-full max-w-screen-md space-y-10 rounded-2xl border-2 border-slate-600/20 bg-white p-8 shadow-2xl md:p-12'>
      {children}
    </div>
  )
}

export default CardContent
