// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { useState } from 'react'
import axios, { AxiosRequestConfig, Method } from 'axios'
import { handleGenericError } from '@/utils/handle-generic-error'

export const useApi = () => {
  const [isLoading, setIsLoading] = useState<boolean>(false)

  const fetchData = async <T, U = undefined>(
    url: string,
    method: Method = 'get',
    data?: U,
    config?: AxiosRequestConfig,
  ): Promise<T | undefined> => {
    setIsLoading(true)
    try {
      const response = method === 'get' ? await axios.get<T>(`${url}`, config) : await axios.post<T>(`${url}`, data, config)
      return response.data
    } catch (error) {
      handleGenericError(`API Error - ${url}`, error as Error)
    } finally {
      setIsLoading(false)
    }
    return undefined
  }

  return { fetchData, isLoading }
}
