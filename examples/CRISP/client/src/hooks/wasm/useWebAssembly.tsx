import { useState, useEffect } from 'react'
import { handleGenericError } from '@/utils/handle-generic-error'
import { useNotificationAlertContext } from '@/context/NotificationAlert'
import { EncryptedVote } from '@/utils/vote'

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
      let startTime = performance.now()
      console.log("Start Time:", startTime);
      worker.postMessage({ type: 'encrypt_vote', data: { voteId, publicKey } })
      worker.onmessage = (event) => {
        const { type, success, encryptedVote, proof, instances, error } = event.data
        let endTime = performance.now()
        console.log(`Time taken: ${endTime - startTime} milliseconds`)
        if (type === 'encrypt_vote') {
          if (success) {
            resolve({
              vote: encryptedVote,
              proof: proof,
              instances: instances
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

