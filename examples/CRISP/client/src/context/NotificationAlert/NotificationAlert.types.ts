import { NotificationAlert } from '@/model/notification.model'
import { ReactNode } from 'react'

export type NotificationAlertContextType = {
  showToast: (toast: NotificationAlert) => void
}

export type NotificationAlertProviderProps = {
  children: ReactNode
}
