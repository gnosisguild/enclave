// Modal.tsx
import React, { useEffect, useRef, useCallback, FC } from 'react'

interface ModalProps {
  show: boolean
  onClose: () => void
  children: React.ReactNode
  className?: string | undefined
}

const Modal: FC<ModalProps> = ({ show, onClose, children, className }) => {
  const modalRef = useRef<HTMLDivElement>(null)

  const closeModal = (e: React.MouseEvent<HTMLDivElement>) => {
    if (modalRef.current === e.target) {
      onClose()
    }
  }

  const keyPress = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape' && show) {
        onClose()
      }
    },
    [show, onClose],
  )

  useEffect(() => {
    document.addEventListener('keydown', keyPress)
    return () => document.removeEventListener('keydown', keyPress)
  }, [keyPress])

  if (!show) return null

  return (
    <div className='fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-20 p-4' onClick={closeModal} ref={modalRef}>
      <div
        className={`relative max-h-[672px] max-w-screen-md overflow-auto rounded-[24px] border-2 border-slate-600/20 bg-white p-6 shadow-2xl md:p-12 ${className ? className : 'w-full'}`}
      >
        {children}
        <button className='absolute right-4 top-4 md:right-8 md:top-8' onClick={onClose}>
          <div className='close-icon' />
        </button>
      </div>
    </div>
  )
}

export default Modal
