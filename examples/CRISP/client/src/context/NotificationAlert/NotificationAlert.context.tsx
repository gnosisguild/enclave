import { createGenericContext } from '@/utils/create-generic-context'
import { NotificationAlertContextType, NotificationAlertProviderProps } from '@/context/NotificationAlert'
import { useCallback, useState } from 'react'
import { NotificationAlert } from '@/model/notification.model'
import ToastAlert from '@/components/ToastAlert'

const [useNotificationAlertContext, NotificationAlertContextProvider] = createGenericContext<NotificationAlertContextType>()

const NotificationAlertProvider = ({ children }: NotificationAlertProviderProps) => {
  const [toast, setToast] = useState<NotificationAlert | null>(null)

  const showToast = useCallback((toast: NotificationAlert) => {
    setToast(toast)
  }, [])

  const closeToast = useCallback(() => {
    setToast(null)
  }, [])

  return (
    <NotificationAlertContextProvider
      value={{
        showToast,
      }}
    >
      {children}
      {toast && <ToastAlert linkUrl={toast.linkUrl} message={toast.message} type={toast.type} onClose={closeToast} />}
    </NotificationAlertContextProvider>
  )
}

export { useNotificationAlertContext, NotificationAlertProvider }
