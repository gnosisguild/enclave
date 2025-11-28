// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// ToastAlert.tsx
import React, { useEffect } from 'react'
import { Link, X, Warning, Info } from '@phosphor-icons/react'

type ToastAlertProps = {
  type: 'success' | 'danger' | 'warning' | 'info'
  linkUrl?: string
  message: string
  onClose: () => void
  persistent?: boolean
  id?: string
}

const ToastAlert: React.FC<ToastAlertProps> = ({ message, type, linkUrl, onClose, persistent = false }) => {
  useEffect(() => {
    if (persistent) return

    const timer = setTimeout(() => {
      onClose()
    }, 5000) // Toast will close after 5 seconds

    return () => clearTimeout(timer) // Clean up the timer
  }, [onClose, persistent])

  const alertStyles = {
    success: {
      container: 'border-lime-600/80 shadow-button-outlined',
      text: 'text-lime-600',
      button: 'text-lime-600 hover:text-lime-700',
      icon: null,
    },
    danger: {
      container: 'border-red-600/80 shadow-danger',
      text: 'text-red-600',
      button: 'text-red-600 hover:text-red-700',
      icon: null,
    },
    warning: {
      container: 'border-amber-500/80 shadow-lg',
      text: 'text-amber-600',
      button: 'text-amber-600 hover:text-amber-700',
      icon: Warning,
    },
    info: {
      container: 'border-blue-500/80 shadow-lg',
      text: 'text-blue-600',
      button: 'text-blue-600 hover:text-blue-700',
      icon: Info,
    },
  }

  const currentAlertStyle = alertStyles[type]
  const IconComponent = currentAlertStyle.icon

  return (
    <div className='toast-alert relative transform transition-transform animate-in slide-in-from-left-5 pointer-events-auto'>
      <div
        className={`shadow-toast min-w-[366px] max-w-[500px] flex items-center rounded-[16px] border-2 ${currentAlertStyle.container} bg-white px-6 py-3`}
      >
        <div className='flex w-full items-center justify-between gap-3'>
          <div className='flex items-center gap-2 flex-1'>
            {IconComponent && <IconComponent size={20} weight='bold' className={currentAlertStyle.text} />}
            {linkUrl ? (
              <a
                href={linkUrl}
                target='_blank'
                rel='noopener noreferrer'
                className={`flex items-center text-base font-extrabold uppercase leading-6 ${currentAlertStyle.text}`}
              >
                <Link size={16} weight='bold' className={`mr-2 ${currentAlertStyle.button}`} />
                {message}
              </a>
            ) : (
              <p className={`text-base font-extrabold uppercase leading-6 ${currentAlertStyle.text}`}>{message}</p>
            )}
          </div>

          <button onClick={onClose} className='flex-shrink-0 ml-2'>
            <X weight='bold' size={16} className={currentAlertStyle.button} />
          </button>
        </div>
        {persistent && (
          <div className='absolute -top-1 -right-1 w-3 h-3 bg-red-500 rounded-full animate-pulse' title='Requires manual dismissal' />
        )}
      </div>
    </div>
  )
}

export default ToastAlert
