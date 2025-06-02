import { useState, useEffect } from 'react'
import { useWriteContract, useWaitForTransactionReceipt, useWatchContractEvent } from 'wagmi'
import { parseEther } from 'viem'
import { ENCLAVE_ADDRESS, ENCLAVE_ABI, E3_PROGRAM_ADDRESS, FILTER_REGISTRY_ADDRESS } from '@/config/Enclave.abi'
import {
    encodeBfvParams,
    encodeComputeProviderParams,
    calculateStartWindow,
    DEFAULT_COMPUTE_PROVIDER_PARAMS,
    DEFAULT_E3_CONFIG
} from '@/utils/bfv-params'
import { useNotificationAlertContext } from '@/context/NotificationAlert'

export interface E3RequestParams {
    threshold?: [number, number]
    windowSize?: number
    duration?: number
    paymentAmount?: string
}

export interface E3State {
    id: bigint | null
    isRequested: boolean
    isActivated: boolean
    publicKey: `0x${string}` | null
    expiresAt: bigint | null
}

export const useEnclaveContract = () => {
    const [e3State, setE3State] = useState<E3State>({
        id: null,
        isRequested: false,
        isActivated: false,
        publicKey: null,
        expiresAt: null
    })
    const { showToast } = useNotificationAlertContext()

    const {
        data: txHash,
        writeContract,
        error: writeError,
        isPending: isWritePending,
    } = useWriteContract()

    const { isLoading: isConfirming, isSuccess: isConfirmed } = useWaitForTransactionReceipt({
        hash: txHash,
    })

    // Listen for E3Requested events
    useWatchContractEvent({
        address: ENCLAVE_ADDRESS as `0x${string}`,
        abi: ENCLAVE_ABI,
        eventName: 'E3Requested',
        onLogs(logs) {
            logs.forEach((log) => {
                const { e3Id } = log.args
                if (e3Id) {
                    console.log('E3Requested event received:', { e3Id: e3Id.toString() })
                    setE3State(prev => ({
                        ...prev,
                        id: e3Id,
                        isRequested: true
                    }))
                    showToast({
                        type: 'success',
                        message: `E3 computation requested! E3 ID: ${e3Id.toString()}`
                    })
                }
            })
        },
    })

    // Listen for E3Activated events
    useWatchContractEvent({
        address: ENCLAVE_ADDRESS as `0x${string}`,
        abi: ENCLAVE_ABI,
        eventName: 'E3Activated',
        onLogs(logs) {
            logs.forEach((log) => {
                const { e3Id, expiresAt, publicKey } = log.args
                if (e3Id && e3State.id && e3Id === e3State.id) {
                    console.log('E3Activated event received for our E3:', {
                        e3Id: e3Id.toString(),
                        expiresAt: expiresAt?.toString(),
                        publicKeyLength: publicKey?.length
                    })
                    setE3State(prev => ({
                        ...prev,
                        isActivated: true,
                        publicKey: publicKey as `0x${string}`,
                        expiresAt: expiresAt || null
                    }))
                    showToast({
                        type: 'success',
                        message: `E3 computation activated! Ready for encrypted inputs.`
                    })
                }
            })
        },
    })

    const requestComputation = async (params: E3RequestParams = {}) => {
        // Reset E3 state
        setE3State({
            id: null,
            isRequested: false,
            isActivated: false,
            publicKey: null,
            expiresAt: null
        })

        try {
            // Prepare parameters with defaults
            const threshold: [number, number] = params.threshold || [DEFAULT_E3_CONFIG.threshold_min, DEFAULT_E3_CONFIG.threshold_max]
            const startWindow = calculateStartWindow(params.windowSize)
            const duration = BigInt(params.duration || DEFAULT_E3_CONFIG.duration)
            // Contract requires msg.value > 0, so minimum payment is 0.001 ETH
            const paymentAmount = params.paymentAmount || "0.001"

            // Encode parameters
            const e3ProgramParams = encodeBfvParams()
            const computeProviderParams = encodeComputeProviderParams(DEFAULT_COMPUTE_PROVIDER_PARAMS)

            console.log('Requesting E3 computation with params:', {
                filter: FILTER_REGISTRY_ADDRESS,
                threshold,
                startWindow: startWindow.map(w => w.toString()),
                duration: duration.toString(),
                e3Program: E3_PROGRAM_ADDRESS,
                e3ProgramParams,
                computeProviderParams,
                paymentAmount
            })

            // Make the contract call
            writeContract({
                address: ENCLAVE_ADDRESS as `0x${string}`,
                abi: ENCLAVE_ABI,
                functionName: 'request',
                args: [
                    FILTER_REGISTRY_ADDRESS as `0x${string}`,
                    threshold as [number, number],
                    startWindow,
                    duration,
                    E3_PROGRAM_ADDRESS as `0x${string}`,
                    e3ProgramParams,
                    computeProviderParams
                ],
                value: parseEther(paymentAmount)
            })

        } catch (error: any) {
            console.error('Failed to request E3 computation:', error)
            showToast({
                type: 'danger',
                message: `Failed to request computation: ${error.message || 'Unknown error'}`
            })
        }
    }

    // Handle successful transaction confirmation
    useEffect(() => {
        if (isConfirmed && txHash) {
            showToast({
                type: 'success',
                message: 'Transaction confirmed! Waiting for E3 events...'
            })
        }
    }, [isConfirmed, txHash, showToast])

    // Handle transaction errors
    useEffect(() => {
        if (writeError) {
            console.error('Transaction error:', writeError)
            showToast({
                type: 'danger',
                message: `Transaction failed: ${writeError.message || 'Unknown error'}`
            })
        }
    }, [writeError, showToast])

    return {
        requestComputation,
        e3State,
        isRequesting: isWritePending || isConfirming,
        isSuccess: isConfirmed,
        error: writeError,
        transactionHash: txHash
    }
} 