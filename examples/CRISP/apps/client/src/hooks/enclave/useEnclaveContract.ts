import { useState, useEffect } from 'react'
import { useWriteContract, useWaitForTransactionReceipt, useWatchContractEvent } from 'wagmi'
import { parseEther } from 'viem'
import { ENCLAVE_ADDRESS, ENCLAVE_ABI, E3_PROGRAM_ADDRESS, REGISTRY_ADDRESS, FILTER_REGISTRY_ADDRESS, REGISTRY_ABI } from '@/config/Enclave.abi'
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
    isCommitteePublished: boolean
    isActivated: boolean
    publicKey: `0x${string}` | null
    expiresAt: bigint | null
}

export const useEnclaveContract = () => {
    const [e3State, setE3State] = useState<E3State>({
        id: null,
        isRequested: false,
        isCommitteePublished: false,
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
        chainId: 31337,
        onLogs(logs) {
            console.log('📥 E3Requested event - received', logs.length, 'logs')
            logs.forEach((log) => {
                const { e3Id } = (log as any).args
                console.log('📥 E3Requested:', { e3Id: e3Id?.toString() })

                if (e3Id) {
                    console.log('✅ Updating E3 state with ID:', e3Id.toString())
                    setE3State(prev => ({
                        ...prev,
                        id: e3Id,
                        isRequested: true
                    }))
                    setTimeout(() => {
                        showToast({
                            type: 'success',
                            message: `E3 computation requested! E3 ID: ${e3Id.toString()}`
                        })
                    }, 0)
                }
            })
        },
        onError(error) {
            console.error('❌ E3Requested event watcher error:', error)
        }
    })

    // Listen for CommitteePublished events from the Registry
    useWatchContractEvent({
        address: REGISTRY_ADDRESS as `0x${string}`,
        abi: REGISTRY_ABI,
        eventName: 'CommitteePublished',
        chainId: 31337,
        onLogs(logs) {
            console.log('📥 CommitteePublished event - received', logs.length, 'logs')
            logs.forEach((log) => {
                const { e3Id, publicKey } = log.args
                console.log('📥 CommitteePublished:', { e3Id: e3Id?.toString() })

                setE3State(prevState => {
                    if (e3Id && prevState.id && e3Id === prevState.id && !prevState.isCommitteePublished) {
                        console.log('✅ Committee published key for our E3!')
                        setTimeout(() => {
                            showToast({
                                type: 'success',
                                message: `Committee published public key! Ready to activate E3.`
                            })
                        }, 0)

                        return {
                            ...prevState,
                            isCommitteePublished: true,
                            publicKey: publicKey as `0x${string}`
                        }
                    }
                    return prevState
                })
            })
        },
        onError(error) {
            console.error('❌ CommitteePublished event watcher error:', error)
        }
    })

    // Listen for E3Activated events
    useWatchContractEvent({
        address: ENCLAVE_ADDRESS as `0x${string}`,
        abi: ENCLAVE_ABI,
        eventName: 'E3Activated',
        chainId: 31337,
        onLogs(logs) {
            console.log('📥 E3Activated event - received', logs.length, 'logs')
            logs.forEach((log) => {
                const { e3Id, expiresAt } = (log as any).args
                if (e3Id && e3State.id && e3Id === e3State.id) {
                    console.log('✅ E3 activated for our E3:', e3Id.toString())
                    setE3State(prev => ({
                        ...prev,
                        isActivated: true,
                        expiresAt: expiresAt || null
                    }))
                    setTimeout(() => {
                        showToast({
                            type: 'success',
                            message: `E3 computation activated! Ready for encrypted inputs.`
                        })
                    }, 0)
                }
            })
        },
        onError(error) {
            console.error('❌ E3Activated event watcher error:', error)
        }
    })

    // Listen for InputPublished events
    useWatchContractEvent({
        address: ENCLAVE_ADDRESS as `0x${string}`,
        abi: ENCLAVE_ABI,
        eventName: 'InputPublished',
        chainId: 31337,
        onLogs(logs) {
            console.log('📥 InputPublished event - received', logs.length, 'logs')
            logs.forEach((log) => {
                const { e3Id, index } = (log as any).args
                if (e3Id && e3State.id && e3Id === e3State.id) {
                    console.log('✅ Input published for our E3:', { e3Id: e3Id.toString(), index: index?.toString() })
                    setTimeout(() => {
                        showToast({
                            type: 'success',
                            message: `Input published successfully! Index: ${index?.toString()}`
                        })
                    }, 0)
                }
            })
        },
        onError(error) {
            console.error('❌ InputPublished event watcher error:', error)
        }
    })

    const requestComputation = async (params: E3RequestParams = {}) => {
        // Reset E3 state
        setE3State({
            id: null,
            isRequested: false,
            isCommitteePublished: false,
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

    const activateE3 = async () => {
        if (!e3State.id || !e3State.publicKey) {
            showToast({
                type: 'danger',
                message: 'Cannot activate: Missing E3 ID or public key'
            })
            return
        }

        try {
            console.log('Activating E3 with params:', {
                e3Id: e3State.id.toString(),
                publicKey: e3State.publicKey
            })

            writeContract({
                address: ENCLAVE_ADDRESS as `0x${string}`,
                abi: ENCLAVE_ABI,
                functionName: 'activate',
                args: [
                    e3State.id,
                    e3State.publicKey
                ]
            })

        } catch (error: any) {
            console.error('Failed to activate E3:', error)
            showToast({
                type: 'danger',
                message: `Failed to activate E3: ${error.message || 'Unknown error'}`
            })
        }
    }

    const publishInput = async (encryptedData: Uint8Array) => {
        if (!e3State.id) {
            showToast({
                type: 'danger',
                message: 'Cannot publish input: Missing E3 ID'
            })
            return
        }

        if (!e3State.isActivated) {
            showToast({
                type: 'danger',
                message: 'Cannot publish input: E3 is not activated'
            })
            return
        }

        try {
            console.log('Publishing input with params:', {
                e3Id: e3State.id.toString(),
                dataLength: encryptedData.length
            })

            // Convert Uint8Array to hex string
            const hexData = `0x${Array.from(encryptedData).map(b => b.toString(16).padStart(2, '0')).join('')}` as `0x${string}`

            writeContract({
                address: ENCLAVE_ADDRESS as `0x${string}`,
                abi: ENCLAVE_ABI,
                functionName: 'publishInput',
                args: [
                    e3State.id,
                    hexData
                ]
            })

        } catch (error: any) {
            console.error('Failed to publish input:', error)
            showToast({
                type: 'danger',
                message: `Failed to publish input: ${error.message || 'Unknown error'}`
            })
        }
    }

    // Handle successful transaction confirmation
    useEffect(() => {
        if (isConfirmed && txHash) {
            console.log('🎉 Transaction confirmed! Hash:', txHash)
            showToast({
                type: 'success',
                message: 'Transaction confirmed! Event watchers should handle the rest.'
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
        activateE3,
        publishInput,
        e3State,
        isRequesting: isWritePending || isConfirming,
        isSuccess: isConfirmed,
        error: writeError,
        transactionHash: txHash
    }
} 