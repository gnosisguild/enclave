// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { useState, useEffect, useRef } from 'react'
import { handleGenericError } from '@/utils/handle-generic-error'
import { useNotificationAlertContext } from '@/context/NotificationAlert'

export const useSDKWorkerHook = () => {
  const { showToast } = useNotificationAlertContext()
  const [isLoading, setIsLoading] = useState<boolean>(false)
  const workerRef = useRef<Worker | null>(null)

  useEffect(() => {
    const newWorker = new Worker(new URL('libs/crispSDKWorker.js', import.meta.url), {
      type: 'module',
    })

    workerRef.current = newWorker

    return () => {
      newWorker.terminate()
    }
  }, [])

  const generateProof = async (
    voteId: bigint,
    publicKey: Uint8Array,
    address: string,
    signature: string,
    previousCiphertext?: Uint8Array,
  ): Promise<string | undefined> => {
    if (!workerRef.current) {
      console.error('Worker not initialized')
      return
    }

    return new Promise<string | undefined>((resolve, reject) => {
      setIsLoading(true)

      workerRef.current!.postMessage({
        type: 'generate_proof',
        data: { voteId, publicKey, address, signature, previousCiphertext },
      })

      workerRef.current!.onmessage = async (event) => {
        const { type, success, encodedProof, error } = event.data

        if (type === 'generate_proof') {
          if (success) {
            resolve(encodedProof)
          } else {
            showToast({
              type: 'danger',
              message: error,
            })

            handleGenericError('generateProof', new Error(error))

            reject(new Error(error))
          }

          setIsLoading(false)
        }
      }
    })
  }

  return {
    isLoading,
    generateProof,
  }
}
