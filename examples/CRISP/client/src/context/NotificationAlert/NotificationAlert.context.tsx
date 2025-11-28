// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { createGenericContext } from '@/utils/create-generic-context'
import { NotificationAlertContextType, NotificationAlertProviderProps } from '@/context/NotificationAlert'
import { useCallback, useState, useMemo } from 'react'
import { NotificationAlert } from '@/model/notification.model'
import ToastAlert from '@/components/ToastAlert'

const MAX_TOASTS = 5
const DEFAULT_DURATION = 5000

const generateToastId = (): string => `toast-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`

const [useNotificationAlertContext, NotificationAlertContextProvider] = createGenericContext<NotificationAlertContextType>()

const NotificationAlertProvider = ({ children }: NotificationAlertProviderProps) => {
  const [toasts, setToasts] = useState<NotificationAlert[]>([])

  const closeToast = useCallback((id?: string) => {
    if (id) {
      setToasts((prev) => prev.filter((t) => t.id !== id))
    } else {
      setToasts((prev) => {
        const nonPersistentIndex = prev.findIndex((t) => !t.persistent)
        if (nonPersistentIndex !== -1) {
          return prev.filter((_, i) => i !== nonPersistentIndex)
        }
        return prev
      })
    }
  }, [])

  const showToast = useCallback(
    (toast: NotificationAlert) => {
      const toastWithId: NotificationAlert = {
        ...toast,
        id: toast.id || generateToastId(),
      }

      setToasts((prev) => {
        const newToasts = prev.length >= MAX_TOASTS ? prev.slice(1) : prev

        return [...newToasts, toastWithId]
      })

      if (!toast.persistent) {
        const duration = toast.duration || DEFAULT_DURATION
        setTimeout(() => {
          closeToast(toastWithId.id)
        }, duration)
      }
    },
    [closeToast],
  )

  const clearAllToasts = useCallback(() => {
    setToasts([])
  }, [])

  const contextValue = useMemo(
    () => ({
      showToast,
      closeToast,
      clearAllToasts,
    }),
    [showToast, closeToast, clearAllToasts],
  )

  return (
    <NotificationAlertContextProvider value={contextValue}>
      {children}
      <div
        className='fixed bottom-8 left-8 z-[9999] flex flex-col-reverse gap-2 pointer-events-none'
        role='region'
        aria-label='Notifications'
        aria-live='polite'
      >
        {toasts.map((toast) => (
          <ToastAlert
            key={toast.id}
            id={toast.id}
            linkUrl={toast.linkUrl}
            message={toast.message}
            type={toast.type}
            persistent={toast.persistent}
            onClose={() => closeToast(toast.id)}
          />
        ))}
      </div>
    </NotificationAlertContextProvider>
  )
}

export { useNotificationAlertContext, NotificationAlertProvider }
