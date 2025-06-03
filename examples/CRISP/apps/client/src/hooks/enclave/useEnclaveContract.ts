import { useState, useEffect, useRef } from 'react'
import { useWriteContract, useWaitForTransactionReceipt, useConfig } from 'wagmi'
import { watchContractEvent } from '@wagmi/core'
import { parseEther } from 'viem'
import { ENCLAVE_ADDRESS, ENCLAVE_ABI, E3_PROGRAM_ADDRESS, REGISTRY_ADDRESS, FILTER_REGISTRY_ADDRESS, REGISTRY_ABI } from '@/config/Enclave.abi'
import {
    encodeBfvParams,
    encodeComputeProviderParams,
    calculateStartWindow,
    DEFAULT_COMPUTE_PROVIDER_PARAMS,
    DEFAULT_E3_CONFIG
} from '@/utils/bfv-params'

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

    const config = useConfig()
    const unsubscribersRef = useRef<Array<() => void>>([])

    const {
        data: txHash,
        writeContract,
        error: writeError,
        isPending: isWritePending,
    } = useWriteContract()

    const { isLoading: isConfirming, isSuccess: isConfirmed } = useWaitForTransactionReceipt({
        hash: txHash,
    })

    // Set up event watchers to listen for blockchain events
    useEffect(() => {
        // Clean up previous watchers
        unsubscribersRef.current.forEach(unsubscribe => unsubscribe())
        unsubscribersRef.current = []

        // Listen for E3Requested events
        const e3RequestedUnsubscribe = watchContractEvent(config, {
            address: ENCLAVE_ADDRESS as `0x${string}`,
            abi: ENCLAVE_ABI,
            eventName: 'E3Requested',
            chainId: 31337,
            onLogs(logs) {
                logs.forEach((log) => {
                    const { e3Id } = (log as any).args
                    if (e3Id) {
                        setE3State(prev => ({
                            ...prev,
                            id: e3Id,
                            isRequested: true
                        }))
                    }
                })
            }
        })

        // Listen for CommitteePublished events from the Registry
        const committeePublishedUnsubscribe = watchContractEvent(config, {
            address: REGISTRY_ADDRESS as `0x${string}`,
            abi: REGISTRY_ABI,
            eventName: 'CommitteePublished',
            chainId: 31337,
            onLogs(logs) {
                logs.forEach((log) => {
                    const { e3Id, publicKey } = (log as any).args
                    setE3State(prevState => {
                        if (e3Id && prevState.id && e3Id === prevState.id && !prevState.isCommitteePublished) {
                            return {
                                ...prevState,
                                isCommitteePublished: true,
                                publicKey: publicKey as `0x${string}`
                            }
                        }
                        return prevState
                    })
                })
            }
        })

        // Listen for E3Activated events
        const e3ActivatedUnsubscribe = watchContractEvent(config, {
            address: ENCLAVE_ADDRESS as `0x${string}`,
            abi: ENCLAVE_ABI,
            eventName: 'E3Activated',
            chainId: 31337,
            onLogs(logs) {
                logs.forEach((log) => {
                    const { e3Id, expiration } = (log as any).args
                    setE3State(prevState => {
                        if (e3Id && prevState.id && e3Id === prevState.id) {
                            return {
                                ...prevState,
                                isActivated: true,
                                expiresAt: expiration || null
                            }
                        }
                        return prevState
                    })
                })
            }
        })

        // Listen for InputPublished events
        const inputPublishedUnsubscribe = watchContractEvent(config, {
            address: ENCLAVE_ADDRESS as `0x${string}`,
            abi: ENCLAVE_ABI,
            eventName: 'InputPublished',
            chainId: 31337,
            onLogs(logs) {
                logs.forEach((log) => {
                    const { e3Id } = (log as any).args
                    // Event captured but no action needed for this tutorial
                })
            }
        })

        // Store unsubscribers for cleanup
        unsubscribersRef.current = [
            e3RequestedUnsubscribe,
            committeePublishedUnsubscribe,
            e3ActivatedUnsubscribe,
            inputPublishedUnsubscribe
        ]

        // Cleanup on unmount
        return () => {
            unsubscribersRef.current.forEach(unsubscribe => unsubscribe())
            unsubscribersRef.current = []
        }
    }, [config])

    const requestComputation = async (params: E3RequestParams = {}) => {
        // Reset E3 state for new request
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
            const paymentAmount = params.paymentAmount || "0.001"

            // Encode parameters for the smart contract
            const e3ProgramParams = encodeBfvParams()
            const computeProviderParams = encodeComputeProviderParams(DEFAULT_COMPUTE_PROVIDER_PARAMS)

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
            console.error('Failed to request computation:', error)
        }
    }

    const activateE3 = async () => {
        if (!e3State.id || !e3State.publicKey) {
            console.error('Cannot activate: Missing E3 ID or public key')
            return
        }

        try {
            writeContract({
                address: ENCLAVE_ADDRESS as `0x${string}`,
                abi: ENCLAVE_ABI,
                functionName: 'activate',
                args: [e3State.id, e3State.publicKey]
            })
        } catch (error: any) {
            console.error('Failed to activate E3:', error)
        }
    }

    const publishInput = async (encryptedData: Uint8Array) => {
        if (!e3State.id) {
            console.error('Cannot publish input: Missing E3 ID')
            return
        }

        if (!e3State.isActivated) {
            console.error('Cannot publish input: E3 is not activated')
            return
        }

        try {
            // Convert Uint8Array to hex string for the contract
            const hexData = `0x${Array.from(encryptedData).map(b => b.toString(16).padStart(2, '0')).join('')}` as `0x${string}`

            writeContract({
                address: ENCLAVE_ADDRESS as `0x${string}`,
                abi: ENCLAVE_ABI,
                functionName: 'publishInput',
                args: [e3State.id, hexData]
            })
        } catch (error: any) {
            console.error('Failed to publish input:', error)
        }
    }

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