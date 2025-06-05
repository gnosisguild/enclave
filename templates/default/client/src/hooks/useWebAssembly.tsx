import { useState, useEffect } from 'react'
import init, { Encrypt } from 'libs/wasm/pkg/wasm_crypto'

export const useWebAssemblyHook = () => {
    const [isLoaded, setIsLoaded] = useState(false)

    useEffect(() => {
        const loadWasm = async () => {
            try {
                await init()
                setIsLoaded(true)
                console.log('WASM module loaded')
            } catch (error) {
                console.error('Failed to load WASM module:', error)
            }
        }
        loadWasm()
    }, [])

    const encryptInput = async (value: bigint, publicKey: Uint8Array): Promise<Uint8Array | null> => {
        if (!isLoaded) {
            console.error('WASM module not loaded yet')
            return null
        }

        try {
            console.log('Encrypting input')
            const encryptor = new Encrypt()
            const result = encryptor.encrypt(value, publicKey)
            console.log('Encrypted input:', result)
            return result
        } catch (error) {
            console.error('Error encrypting input:', error)
            return null
        }
    }

    return {
        isLoaded,
        encryptInput
    }
} 