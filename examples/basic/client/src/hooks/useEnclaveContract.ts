import { useState, useEffect, useRef } from 'react'
import { useWriteContract, useWaitForTransactionReceipt, useConfig } from 'wagmi'
import { watchContractEvent } from '@wagmi/core'
import { parseEther, bytesToHex } from 'viem'
import { ENCLAVE_ADDRESS, ENCLAVE_ABI, E3_PROGRAM_ADDRESS, REGISTRY_ADDRESS, FILTER_REGISTRY_ADDRESS, REGISTRY_ABI } from '@/utils/enclave.config'
import {
    encodeBfvParams,
    encodeComputeProviderParams,
    calculateStartWindow,
    DEFAULT_COMPUTE_PROVIDER_PARAMS,
    DEFAULT_E3_CONFIG
} from '@/utils/bfv'

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
    plaintextOutput: string | null
    hasPlaintextOutput: boolean
}

export const useEnclaveContract = () => {
    const [e3State, setE3State] = useState<E3State>({
        id: null,
        isRequested: false,
        isCommitteePublished: false,
        isActivated: false,
        publicKey: null,
        expiresAt: null,
        plaintextOutput: null,
        hasPlaintextOutput: false
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

        // Listen for PlaintextOutputPublished events
        const plaintextOutputUnsubscribe = watchContractEvent(config, {
            address: ENCLAVE_ADDRESS as `0x${string}`,
            abi: ENCLAVE_ABI,
            eventName: 'PlaintextOutputPublished',
            chainId: 31337,
            onLogs(logs) {
                logs.forEach((log) => {
                    const { e3Id, plaintextOutput } = (log as any).args
                    setE3State(prevState => {
                        if (e3Id && prevState.id && e3Id === prevState.id) {
                            return {
                                ...prevState,
                                plaintextOutput: plaintextOutput as string,
                                hasPlaintextOutput: true
                            }
                        }
                        return prevState
                    })
                })
            }
        })

        // Store unsubscribers for cleanup
        unsubscribersRef.current = [
            e3RequestedUnsubscribe,
            committeePublishedUnsubscribe,
            e3ActivatedUnsubscribe,
            plaintextOutputUnsubscribe
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
            expiresAt: null,
            plaintextOutput: null,
            hasPlaintextOutput: false
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
            const hexData = bytesToHex(encryptedData)

            writeContract({
                address: ENCLAVE_ADDRESS as `0x${string}`,
                abi: ENCLAVE_ABI,
                functionName: 'publishInput',
                args: [e3State.id, hexData],
                gas: 2000000n
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