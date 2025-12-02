// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { NotificationAlert } from '@/model/notification.model'
import { ReactNode } from 'react'

export type NotificationAlertContextType = {
  showToast: (toast: NotificationAlert) => void
  closeToast: (id?: string) => void
  clearAllToasts: () => void
}

export type NotificationAlertProviderProps = {
  children: ReactNode
}
