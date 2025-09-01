// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// ToastAlert.tsx
import React, { useEffect } from 'react'
import { Link, X } from '@phosphor-icons/react'

type ToastAlertProps = {
  type: 'success' | 'danger'
  linkUrl?: string
  message: string
  onClose: () => void
}

const ToastAlert: React.FC<ToastAlertProps> = ({ message, type, linkUrl, onClose }) => {
  useEffect(() => {
    const timer = setTimeout(() => {
      onClose()
    }, 5000) // Toast will close after 5 seconds

    return () => clearTimeout(timer) // Clean up the timer
  }, [onClose])

  const alertStyles = {
    success: {
      container: 'border-lime-600/80 shadow-button-outlined',
      text: 'text-lime-600',
      button: 'text-lime-600 hover:text-lime-700',
    },
    danger: {
      container: 'border-red-600/80 shadow-danger',
      text: 'text-red-600',
      button: 'text-red-600 hover:text-red-700',
    },
  }

  const currentAlertStyle = alertStyles[type]

  return (
    <div className='toast-alert fixed bottom-8 left-8 z-[9999] transform transition-transform'>
      <div
        className={`shadow-toast w-min-[366px] flex h-[46px] items-center rounded-[16px] border-2 ${currentAlertStyle.container} bg-white px-6`}
      >
        <div className='flex w-full items-center justify-between'>
          {linkUrl && (
            <a
              href={linkUrl}
              target='_blank'
              className={`mr-6 flex items-center text-base font-extrabold uppercase leading-6 ${currentAlertStyle.text}`}
            >
              <Link size={16} weight='bold' className={`mr-2 ${currentAlertStyle.button}`} />
              {message}
            </a>
          )}
          {!linkUrl && <p className={`mr-3 text-base font-extrabold uppercase leading-6 ${currentAlertStyle.text}`}>{message}</p>}

          <button onClick={onClose}>
            <X weight='bold' size={16} className={currentAlertStyle.button} />
          </button>
        </div>
      </div>
    </div>
  )
}

export default ToastAlert
