// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { useState, useEffect } from 'react'
import { handleGenericError } from '@/utils/handle-generic-error'
import { useNotificationAlertContext } from '@/context/NotificationAlert'
import { EncryptedVote } from '@/model/vote.model'
import {
  generateProof,
  CircuitInputs,
} from '@/utils/proofUtils'

export const useWebAssemblyHook = () => {
  const { showToast } = useNotificationAlertContext()
  const [isLoading, setIsLoading] = useState<boolean>(false)
  const [worker, setWorker] = useState<Worker | null>(null)

  useEffect(() => {
    const newWorker = new Worker(new URL('libs/wasm/pkg/crisp_worker.js', import.meta.url), {
      type: 'module',
    })
    setWorker(newWorker)
    return () => {
      newWorker.terminate()
    }
  }, [])

  const encryptVote = async (voteId: bigint, publicKey: Uint8Array): Promise<EncryptedVote | undefined> => {
    if (!worker) {
      console.error('WebAssembly worker not initialized')
      return
    }

    return new Promise<EncryptedVote | undefined>((resolve, reject) => {
      setIsLoading(true)
      worker.postMessage({ type: 'encrypt_vote', data: { voteId, publicKey } })
      worker.onmessage = async (event) => {
        const { type, success, encryptedVote, error } = event.data
        if (type === 'encrypt_vote') {
          if (success) {
            const { vote, circuitInputs } = encryptedVote;
            const { proof, publicInputs } = await generateProof(circuitInputs as CircuitInputs);
            resolve({
              vote: vote,
              proof: proof,
              public_inputs: publicInputs,
            })
          } else {
            showToast({
              type: 'danger',
              message: error,
            })
            handleGenericError('encryptVote', new Error(error))
            reject(new Error(error))
          }
          setIsLoading(false)
        }
      }
    })
  }

  return {
    isLoading,
    encryptVote,
  }
}
