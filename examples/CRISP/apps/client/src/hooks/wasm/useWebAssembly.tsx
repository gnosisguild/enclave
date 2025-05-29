import { useState, useEffect } from 'react'
import { handleGenericError } from '@/utils/handle-generic-error'
import { useNotificationAlertContext } from '@/context/NotificationAlert'
import { UltraHonkBackend } from '@aztec/bb.js'
import { Noir } from '@noir-lang/noir_js'
import crisp_circuit from 'libs/noir/crisp_circuit.json'

const generateProof = async (x: number, y: number) => {
  const noir = new Noir(crisp_circuit as any)
  console.log('Starting execution')
  const backend = new UltraHonkBackend((crisp_circuit as any).bytecode, { threads: 4 })
  const { witness } = await noir.execute({ x, y })
  console.log('Generating proof')
  const { proof } = await backend.generateProof(witness)
  console.log('Proof', proof)
  return proof
}

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

  const encryptVote = async (voteId: bigint, publicKey: Uint8Array): Promise<Uint8Array | undefined> => {
    if (!worker) {
      console.error('WebAssembly worker not initialized')
      return
    }

    return new Promise<Uint8Array | undefined>((resolve, reject) => {
      setIsLoading(true)
      worker.postMessage({ type: 'encrypt_vote', data: { voteId, publicKey } })
      worker.onmessage = async (event) => {
        const { type, success, encryptedVote, error } = event.data
        if (type === 'encrypt_vote') {
          if (success) {
            await generateProof(1, 2)
            resolve(encryptedVote)
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
