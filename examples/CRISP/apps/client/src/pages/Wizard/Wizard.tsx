import React, { useState, useEffect } from 'react'
import { useAccount } from 'wagmi'
import { ConnectKitButton } from 'connectkit'
import { hexToBytes } from 'viem'

// Components
import CardContent from '@/components/Cards/CardContent'
import CircularTiles from '@/components/CircularTiles'
import LoadingAnimation from '@/components/LoadingAnimation'
import EnvironmentError from '@/components/EnvironmentError'

// Hooks
import { useEnclaveContract } from '@/hooks/enclave/useEnclaveContract'
import { useWebAssemblyHook } from '@/hooks/wasm/useWebAssembly'

// Config & Utils
import { HAS_MISSING_ENV_VARS, MISSING_ENV_VARS } from '@/config/Enclave.abi'

// Icons
import {
    Wallet,
    Calculator,
    Lock,
    CheckCircle,
    NumberSquareOne,
    NumberSquareTwo,
    NumberSquareThree,
    NumberSquareFour,
    NumberSquareFive,
    NumberSquareSix
} from '@phosphor-icons/react'

// ============================================================================
// TYPES & ENUMS
// ============================================================================

enum WizardStep {
    CONNECT_WALLET = 1,
    REQUEST_COMPUTATION = 2,
    ACTIVATE_E3 = 3,
    ENTER_INPUTS = 4,
    ENCRYPT_SUBMIT = 5,
    RESULTS = 6,
}

// ============================================================================
// STEP COMPONENTS
// ============================================================================

const ConnectWalletStep: React.FC = () => (
    <CardContent>
        <div className='space-y-6 text-center'>
            <div className="flex justify-center">
                <Wallet size={48} className="text-lime-400" />
            </div>
            <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 1: Connect Your Wallet</p>
            <div className="space-y-4">
                <h3 className="text-lg font-semibold text-slate-700">Welcome to Encrypted Computation</h3>
                <p className="text-slate-600 leading-relaxed">
                    To begin the encrypted computation process, you'll need to connect your wallet. This enables secure
                    cryptographic operations and ensures your privacy throughout the computation.
                </p>
                <div className="bg-lime-50 border border-lime-200 rounded-lg p-4">
                    <p className="text-sm text-slate-600">
                        <strong>What happens next:</strong> After connecting, you'll request a computation session,
                        wait for committee activation, activate the E3, enter two numbers, and see them encrypted
                        before being published to the secure computation environment.
                    </p>
                </div>
            </div>
            <div className="pt-4 flex justify-center">
                <ConnectKitButton />
            </div>
        </div>
    </CardContent>
)

interface RequestComputationStepProps {
    e3State: any
    isRequesting: boolean
    transactionHash: string | undefined
    error: any
    isSuccess: boolean
    onRequestComputation: () => Promise<void>
}

const RequestComputationStep: React.FC<RequestComputationStepProps> = ({
    e3State,
    isRequesting,
    transactionHash,
    error,
    isSuccess,
    onRequestComputation
}) => (
    <CardContent>
        <div className='space-y-6 text-center'>
            <div className="flex justify-center">
                <Calculator size={48} className="text-lime-400" />
            </div>
            <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 2: Request Computation</p>
            <div className="space-y-4">
                <h3 className="text-lg font-semibold text-slate-700">Initialize Encrypted Execution Environment</h3>
                <p className="text-slate-600 leading-relaxed">
                    Request an E3 computation from the Enclave network. This will create a secure
                    computation environment and wait for the committee to activate it with a public key.
                </p>
                <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
                    <p className="text-sm text-slate-600">
                        <strong>What happens:</strong> Request → Committee Selection → Public Key → Ready for Activation
                    </p>
                </div>

                {/* E3 State Progress */}
                {e3State.id !== null && (
                    <div className="space-y-3">
                        <div className="bg-green-50 border border-green-200 rounded-lg p-4">
                            <p className="text-sm text-slate-600">
                                <strong>✅ E3 ID:</strong> {String(e3State.id)}
                                <br />
                                <strong>Status:</strong> Computation requested
                            </p>
                        </div>

                        {e3State.isCommitteePublished && e3State.publicKey ? (
                            <div className="bg-lime-50 border border-lime-200 rounded-lg p-4">
                                <p className="text-sm text-slate-600">
                                    <strong>🔑 Committee Published Public Key!</strong>
                                    <br />
                                    <strong>Public Key:</strong> {e3State.publicKey.slice(0, 20)}...{e3State.publicKey.slice(-10)}
                                    <br />
                                    Ready to activate E3 environment.
                                </p>
                            </div>
                        ) : (
                            <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-4">
                                <div className="flex flex-col items-center space-x-2">
                                    <LoadingAnimation isLoading={true} className="!h-6 !w-6" />
                                    <p className="text-sm text-slate-600">
                                        <strong>⏳ Waiting for committee to publish public key...</strong>
                                        <br />
                                        The computation committee is being selected and will provide the public key shortly.
                                    </p>
                                </div>
                            </div>
                        )}
                    </div>
                )}

                {error && (
                    <div className="bg-red-50 border border-red-200 rounded-lg p-4">
                        <p className="text-sm text-red-600">
                            <strong>Error:</strong> {error.message}
                        </p>
                    </div>
                )}

                {isSuccess && transactionHash && (
                    <div className="bg-green-50 border border-green-200 rounded-lg p-4">
                        <p className="text-sm text-green-600">
                            <strong>✅ Transaction Successful!</strong>
                            <br />
                            Hash: {transactionHash.slice(0, 10)}...{transactionHash.slice(-8)}
                        </p>
                    </div>
                )}
            </div>

            <button
                onClick={onRequestComputation}
                disabled={isRequesting || e3State.isRequested}
                className="w-full rounded-lg bg-lime-400 px-6 py-3 font-semibold text-slate-800 transition-all duration-200 hover:bg-lime-300 hover:shadow-md disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-500"
            >
                {isRequesting
                    ? 'Requesting Computation...'
                    : e3State.isRequested
                        ? e3State.isCommitteePublished
                            ? 'Committee Ready - Proceeding to Activation!'
                            : 'Waiting for Committee...'
                        : 'Request E3 Computation (0.001 ETH)'}
            </button>

            {isRequesting && (
                <div className="mt-4">
                    <LoadingAnimation isLoading={isRequesting} />
                    <p className="text-sm text-slate-500 mt-2">Submitting to blockchain...</p>
                </div>
            )}
        </div>
    </CardContent>
)

interface ActivateE3StepProps {
    e3State: any
    isRequesting: boolean
    transactionHash: string | undefined
    error: any
    isSuccess: boolean
    onActivateE3: () => Promise<void>
}

const ActivateE3Step: React.FC<ActivateE3StepProps> = ({
    e3State,
    isRequesting,
    transactionHash,
    error,
    isSuccess,
    onActivateE3
}) => (
    <CardContent>
        <div className='space-y-6 text-center'>
            <div className="flex justify-center">
                <Lock size={48} className="text-lime-400" />
            </div>
            <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 3: Activate E3</p>
            <div className="space-y-4">
                <h3 className="text-lg font-semibold text-slate-700">Activate the Computation Environment</h3>
                <p className="text-slate-600 leading-relaxed">
                    The committee has published their public key. Now you need to activate the E3 environment
                    which will allow it to accept encrypted inputs until it expires.
                </p>
                <div className="bg-green-50 border border-green-200 rounded-lg p-4">
                    <h4 className="font-medium text-slate-700 mb-2">🔒 Committee Ready</h4>
                    <div className="text-sm text-slate-600 space-y-1">
                        <p><strong>E3 ID:</strong> {e3State.id !== null ? String(e3State.id) : 'N/A'}</p>
                        {e3State.publicKey && (
                            <p><strong>Public Key:</strong> {e3State.publicKey.slice(0, 16)}...{e3State.publicKey.slice(-8)}</p>
                        )}
                    </div>
                </div>

                <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
                    <p className="text-sm text-slate-600">
                        <strong>What activation does:</strong> Activating the E3 sets an expiration time and
                        enables the environment to accept encrypted inputs from authorized users.
                    </p>
                </div>

                {error && (
                    <div className="bg-red-50 border border-red-200 rounded-lg p-4">
                        <p className="text-sm text-red-600">
                            <strong>Error:</strong> {error.message}
                        </p>
                    </div>
                )}

                {isSuccess && transactionHash && e3State.isActivated && (
                    <div className="bg-green-50 border border-green-200 rounded-lg p-4">
                        <p className="text-sm text-green-600">
                            <strong>✅ E3 Successfully Activated!</strong>
                            <br />
                            Transaction: {transactionHash.slice(0, 10)}...{transactionHash.slice(-8)}
                            <br />
                            Environment is now ready to accept encrypted inputs.
                        </p>
                    </div>
                )}
            </div>

            <button
                onClick={onActivateE3}
                disabled={isRequesting || e3State.isActivated || !e3State.publicKey}
                className="w-full rounded-lg bg-lime-400 px-6 py-3 font-semibold text-slate-800 transition-all duration-200 hover:bg-lime-300 hover:shadow-md disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-500"
            >
                {isRequesting
                    ? 'Activating E3...'
                    : e3State.isActivated
                        ? 'E3 Activated!'
                        : !e3State.publicKey
                            ? 'Waiting for Public Key...'
                            : 'Activate E3 Environment'}
            </button>

            {isRequesting && (
                <div className="mt-4">
                    <LoadingAnimation isLoading={isRequesting} />
                    <p className="text-sm text-slate-500 mt-2">Activating E3...</p>
                </div>
            )}
        </div>
    </CardContent>
)

interface EnterInputsStepProps {
    e3State: any
    input1: string
    input2: string
    onInput1Change: (value: string) => void
    onInput2Change: (value: string) => void
    onSubmit: (e: React.FormEvent) => void
}

const EnterInputsStep: React.FC<EnterInputsStepProps> = ({
    e3State,
    input1,
    input2,
    onInput1Change,
    onInput2Change,
    onSubmit
}) => (
    <CardContent>
        <div className='space-y-6'>
            <div className="text-center">
                <div className="flex justify-center mb-4">
                    <Lock size={48} className="text-lime-400" />
                </div>
                <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 4: Enter Your Numbers</p>
                <h3 className="text-lg font-semibold text-slate-700 mt-2">Input Data for Encrypted Computation</h3>
                <p className="text-slate-600 mt-2">
                    Enter two numbers that will be encrypted using the committee's public key and published to the E3 environment.
                </p>
            </div>

            {/* E3 Environment Info */}
            <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
                <h4 className="font-medium text-slate-700 mb-2">🔒 Active E3 Environment</h4>
                <div className="text-sm text-slate-600 space-y-1">
                    <p><strong>E3 ID:</strong> {e3State.id !== null ? String(e3State.id) : 'N/A'}</p>
                    {e3State.publicKey && (
                        <p><strong>Encryption Key:</strong> {e3State.publicKey.slice(0, 16)}...{e3State.publicKey.slice(-8)}</p>
                    )}
                    {e3State.expiresAt !== null && (
                        <p><strong>Valid Until:</strong> {new Date(Number(e3State.expiresAt) * 1000).toLocaleString()}</p>
                    )}
                    <p><strong>Status:</strong> {e3State.isActivated ? '✅ Activated & Ready for Inputs' : '⏳ Activating...'}</p>
                </div>
            </div>

            <form onSubmit={onSubmit} className='space-y-6'>
                <div className='space-y-2'>
                    <label htmlFor='input1' className='block text-sm font-medium text-slate-700'>
                        First Number
                    </label>
                    <input
                        type='number'
                        id='input1'
                        value={input1}
                        onChange={(e) => onInput1Change(e.target.value)}
                        placeholder='Enter your first number...'
                        className='w-full rounded-lg border-2 border-slate-300 px-4 py-3 text-slate-700 transition-all duration-200 focus:border-lime-400 focus:outline-none focus:ring-2 focus:ring-lime-400/20 focus:scale-[1.02]'
                        required
                        min="0"
                        max="999999"
                    />
                </div>

                <div className='space-y-2'>
                    <label htmlFor='input2' className='block text-sm font-medium text-slate-700'>
                        Second Number
                    </label>
                    <input
                        type='number'
                        id='input2'
                        value={input2}
                        onChange={(e) => onInput2Change(e.target.value)}
                        placeholder='Enter your second number...'
                        className='w-full rounded-lg border-2 border-slate-300 px-4 py-3 text-slate-700 transition-all duration-200 focus:border-lime-400 focus:outline-none focus:ring-2 focus:ring-lime-400/20 focus:scale-[1.02]'
                        required
                        min="0"
                        max="999999"
                    />
                </div>

                <div className="bg-amber-50 border border-amber-200 rounded-lg p-4">
                    <p className="text-sm text-slate-600">
                        <strong>Privacy Guarantee:</strong> These numbers will be encrypted using the committee's public key
                        from E3 ID {e3State.id !== null ? String(e3State.id) : 'N/A'}, ensuring they remain completely private
                        throughout the homomorphic computation process and will be published to the blockchain.
                    </p>
                </div>

                <button
                    type='submit'
                    disabled={!input1.trim() || !input2.trim() || !e3State.isActivated}
                    className='w-full rounded-lg bg-lime-400 px-6 py-3 font-semibold text-slate-800 transition-all duration-200 hover:bg-lime-300 hover:shadow-md hover:scale-[1.02] disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-500 disabled:transform-none'
                >
                    {!e3State.isActivated ? 'Waiting for E3 Activation...' : 'Encrypt & Publish Numbers'}
                </button>
            </form>
        </div>
    </CardContent>
)

interface EncryptSubmitStepProps {
    inputPublishError: string | null
    inputPublishSuccess: boolean
    onTryAgain: () => void
}

const EncryptSubmitStep: React.FC<EncryptSubmitStepProps> = ({
    inputPublishError,
    inputPublishSuccess,
    onTryAgain
}) => (
    <CardContent>
        <div className='space-y-6 text-center'>
            <div className="flex justify-center">
                <Lock size={48} className="text-lime-400 animate-pulse" />
            </div>
            <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 5: Encrypting & Publishing</p>
            <div className="space-y-4">
                <h3 className="text-lg font-semibold text-slate-700">Processing Your Encrypted Data</h3>
                <p className="text-slate-600 leading-relaxed">
                    Your numbers are being encrypted using homomorphic encryption and published to the secure computation environment.
                    Each input is published as a separate transaction to the blockchain.
                </p>
                <div className="bg-green-50 border border-green-200 rounded-lg p-4">
                    <p className="text-sm text-slate-600">
                        <strong>Processing Steps:</strong>
                        <br />• Encrypting your inputs with FHE
                        <br />• Publishing encrypted input 1 to blockchain
                        <br />• Publishing encrypted input 2 to blockchain
                        <br />• Preparing secure result
                    </p>
                </div>

                {inputPublishError && (
                    <div className="bg-red-50 border border-red-200 rounded-lg p-4">
                        <p className="text-sm text-red-600">
                            <strong>❌ Publishing Failed:</strong>
                            <br />
                            {inputPublishError}
                        </p>
                    </div>
                )}

                {inputPublishSuccess && (
                    <div className="bg-green-50 border border-green-200 rounded-lg p-4">
                        <p className="text-sm text-green-600">
                            <strong>✅ Inputs Successfully Published!</strong>
                            <br />
                            Both encrypted inputs have been submitted to the blockchain.
                        </p>
                    </div>
                )}
            </div>
            <div className="pt-4">
                <LoadingAnimation isLoading={true} />
                <p className="text-sm text-slate-500 mt-4">This may take a moment...</p>
            </div>

            {inputPublishError && (
                <div className="pt-4">
                    <button
                        onClick={onTryAgain}
                        className="w-full rounded-lg bg-slate-600 px-6 py-3 font-semibold text-white transition-all duration-200 hover:bg-slate-500 hover:shadow-md"
                    >
                        Try Again
                    </button>
                </div>
            )}
        </div>
    </CardContent>
)

interface ResultsStepProps {
    input1: string
    input2: string
    result: number | null
    e3State: any
    transactionHash: string | undefined
    onReset: () => void
}

const ResultsStep: React.FC<ResultsStepProps> = ({
    input1,
    input2,
    result,
    e3State,
    transactionHash,
    onReset
}) => (
    <CardContent>
        <div className='space-y-6 text-center'>
            <div className="flex justify-center">
                <CheckCircle size={48} className="text-green-500" />
            </div>
            <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 6: Computation Complete</p>

            <div className="space-y-6">
                <h3 className="text-lg font-semibold text-slate-700">Encrypted Computation Results</h3>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div className="bg-slate-50 border border-slate-200 rounded-lg p-4">
                        <h4 className="font-medium text-slate-700 mb-2">Your Inputs</h4>
                        <p className="text-slate-600">First Number: <span className="font-mono font-semibold">{input1}</span></p>
                        <p className="text-slate-600">Second Number: <span className="font-mono font-semibold">{input2}</span></p>
                    </div>

                    <div className="bg-lime-50 border border-lime-200 rounded-lg p-4">
                        <h4 className="font-medium text-slate-700 mb-2">Computed Result</h4>
                        <p className="text-2xl font-bold text-lime-600">{result}</p>
                        <p className="text-sm text-slate-600">Sum of encrypted inputs</p>
                    </div>
                </div>

                {/* E3 Environment Details */}
                <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
                    <h4 className="font-medium text-slate-700 mb-2">🔒 E3 Computation Details</h4>
                    <div className="text-sm text-slate-600 space-y-1">
                        <p><strong>E3 ID:</strong> {e3State.id !== null ? String(e3State.id) : 'N/A'}</p>
                        {e3State.publicKey && (
                            <p><strong>Encryption Key Used:</strong> {e3State.publicKey.slice(0, 16)}...{e3State.publicKey.slice(-8)}</p>
                        )}
                        {transactionHash && (
                            <p><strong>Request Transaction:</strong> {transactionHash.slice(0, 10)}...{transactionHash.slice(-8)}</p>
                        )}
                        <p><strong>Status:</strong> ✅ Inputs Published to Blockchain</p>
                    </div>
                </div>

                <div className="bg-green-50 border border-green-200 rounded-lg p-4">
                    <h4 className="font-medium text-slate-700 mb-2">What Just Happened?</h4>
                    <p className="text-sm text-slate-600 leading-relaxed">
                        Your numbers were encrypted using the committee's public key from E3 environment {e3State.id !== null ? String(e3State.id) : 'N/A'},
                        published as encrypted inputs to the blockchain, and are now ready for secure computation by the committee
                        without ever revealing your original numbers to the computing system.
                    </p>
                </div>

                <button
                    onClick={onReset}
                    className='w-full rounded-lg bg-slate-600 px-6 py-3 font-semibold text-white transition-all duration-200 hover:bg-slate-500 hover:shadow-md hover:scale-[1.02]'
                >
                    Try Another Computation
                </button>
            </div>
        </div>
    </CardContent>
)

// ============================================================================
// MAIN WIZARD COMPONENT
// ============================================================================

const Wizard: React.FC = () => {
    // ========================================================================
    // ENVIRONMENT CHECK
    // ========================================================================
    if (HAS_MISSING_ENV_VARS) {
        return <EnvironmentError missingVars={MISSING_ENV_VARS} />
    }

    // ========================================================================
    // STATE MANAGEMENT
    // ========================================================================

    // Wizard flow state
    const [currentStep, setCurrentStep] = useState<WizardStep>(WizardStep.CONNECT_WALLET)

    // Input state
    const [input1, setInput1] = useState<string>('')
    const [input2, setInput2] = useState<string>('')

    // Processing state
    const [isLoading, setIsLoading] = useState<boolean>(false)
    const [encryptedInputs, setEncryptedInputs] = useState<{ input1: Uint8Array; input2: Uint8Array } | undefined>(undefined)
    const [result, setResult] = useState<number | null>(null)

    // Error and success state
    const [inputPublishError, setInputPublishError] = useState<string | null>(null)
    const [inputPublishSuccess, setInputPublishSuccess] = useState<boolean>(false)

    // ========================================================================
    // HOOKS
    // ========================================================================

    const { encryptInput } = useWebAssemblyHook()
    const { isConnected } = useAccount()
    const { requestComputation, activateE3, publishInput, e3State, isRequesting, isSuccess, error, transactionHash } = useEnclaveContract()

    // ========================================================================
    // EFFECTS - Wallet Connection Management
    // ========================================================================

    // Auto-advance from step 1 when wallet connects
    useEffect(() => {
        if (isConnected && currentStep === WizardStep.CONNECT_WALLET) {
            setCurrentStep(WizardStep.REQUEST_COMPUTATION)
        }
    }, [isConnected, currentStep])

    // Auto-move back to step 1 if wallet disconnects
    useEffect(() => {
        if (!isConnected && currentStep > WizardStep.CONNECT_WALLET) {
            setCurrentStep(WizardStep.CONNECT_WALLET)
        }
    }, [isConnected, currentStep])

    // ========================================================================
    // EFFECTS - E3 Lifecycle Management
    // ========================================================================

    // Auto-advance when committee publishes public key
    useEffect(() => {
        if (e3State.isCommitteePublished && e3State.publicKey && currentStep === WizardStep.REQUEST_COMPUTATION) {
            setCurrentStep(WizardStep.ACTIVATE_E3)
        }
    }, [e3State.isCommitteePublished, e3State.publicKey, currentStep])

    // Auto-advance when E3 is activated
    useEffect(() => {
        if (e3State.isActivated && currentStep === WizardStep.ACTIVATE_E3) {
            setCurrentStep(WizardStep.ENTER_INPUTS)
        }
    }, [e3State.isActivated, currentStep])

    // ========================================================================
    // EVENT HANDLERS
    // ========================================================================

    const handleRequestComputation = async () => {
        try {
            await requestComputation({
                paymentAmount: "0.001"
            })
        } catch (error) {
            console.error('Failed to request computation:', error)
        }
    }

    const handleActivateE3 = async () => {
        try {
            await activateE3()
        } catch (error) {
            console.error('Failed to activate E3:', error)
        }
    }

    const handleInputSubmit = async (e: React.FormEvent) => {
        e.preventDefault()
        if (!input1.trim() || !input2.trim()) return

        setCurrentStep(WizardStep.ENCRYPT_SUBMIT)
        setIsLoading(true)
        setInputPublishError(null)
        setInputPublishSuccess(false)

        try {
            // UI feedback delay
            await new Promise(resolve => setTimeout(resolve, 1500))

            // Encrypt inputs
            const publicKeyBytes = hexToBytes(e3State.publicKey as `0x${string}`)
            const encryptedInput1 = await encryptInput(BigInt(input1), publicKeyBytes)
            const encryptedInput2 = await encryptInput(BigInt(input2), publicKeyBytes)

            if (!encryptedInput1 || !encryptedInput2) {
                throw new Error('Encryption failed')
            }

            setEncryptedInputs({ input1: encryptedInput1, input2: encryptedInput2 })

            // Publish inputs
            await publishInput(encryptedInput1)
            await new Promise(resolve => setTimeout(resolve, 1000))
            await publishInput(encryptedInput2)

            // Calculate result
            const sum = parseInt(input1) + parseInt(input2)
            setResult(sum)

            // Final processing delay
            await new Promise(resolve => setTimeout(resolve, 1000))

            setInputPublishSuccess(true)
            setIsLoading(false)
            setCurrentStep(WizardStep.RESULTS)
        } catch (error) {
            console.error('Encryption or publishing failed:', error)
            setInputPublishError(error instanceof Error ? error.message : 'Unknown error occurred')
            setIsLoading(false)
        }
    }

    const handleReset = () => {
        setCurrentStep(WizardStep.REQUEST_COMPUTATION)
        setInput1('')
        setInput2('')
        setEncryptedInputs(undefined)
        setResult(null)
        setInputPublishError(null)
        setInputPublishSuccess(false)
    }

    const handleTryAgain = () => {
        setCurrentStep(WizardStep.ENTER_INPUTS)
        setInputPublishError(null)
        setIsLoading(false)
    }

    // ========================================================================
    // UTILITY FUNCTIONS
    // ========================================================================

    const getStepIcon = (step: WizardStep) => {
        const iconProps = { size: 20, weight: 'bold' as const }
        switch (step) {
            case WizardStep.CONNECT_WALLET: return <NumberSquareOne {...iconProps} />
            case WizardStep.REQUEST_COMPUTATION: return <NumberSquareTwo {...iconProps} />
            case WizardStep.ACTIVATE_E3: return <NumberSquareThree {...iconProps} />
            case WizardStep.ENTER_INPUTS: return <NumberSquareFour {...iconProps} />
            case WizardStep.ENCRYPT_SUBMIT: return <NumberSquareFive {...iconProps} />
            case WizardStep.RESULTS: return <NumberSquareSix {...iconProps} />
        }
    }

    const renderStepIndicator = () => (
        <div className="flex items-center justify-center space-x-2 mb-8">
            {[1, 2, 3, 4, 5, 6].map((step) => (
                <div key={step} className="flex items-center">
                    <div
                        className={`flex items-center justify-center w-10 h-10 rounded-full border-2 transition-all duration-300 ${step <= currentStep
                            ? 'bg-lime-400 border-lime-400 text-slate-800'
                            : 'bg-slate-100 border-slate-300 text-slate-500'
                            }`}
                    >
                        {getStepIcon(step as WizardStep)}
                    </div>
                    {step < 6 && (
                        <div
                            className={`w-12 h-0.5 mx-2 transition-all duration-300 ${step < currentStep ? 'bg-lime-400' : 'bg-slate-300'
                                }`}
                        />
                    )}
                </div>
            ))}
        </div>
    )

    // ========================================================================
    // STEP ROUTING
    // ========================================================================

    const renderStepContent = () => {
        switch (currentStep) {
            case WizardStep.CONNECT_WALLET:
                return <ConnectWalletStep />

            case WizardStep.REQUEST_COMPUTATION:
                return (
                    <RequestComputationStep
                        e3State={e3State}
                        isRequesting={isRequesting}
                        transactionHash={transactionHash}
                        error={error}
                        isSuccess={isSuccess}
                        onRequestComputation={handleRequestComputation}
                    />
                )

            case WizardStep.ACTIVATE_E3:
                return (
                    <ActivateE3Step
                        e3State={e3State}
                        isRequesting={isRequesting}
                        transactionHash={transactionHash}
                        error={error}
                        isSuccess={isSuccess}
                        onActivateE3={handleActivateE3}
                    />
                )

            case WizardStep.ENTER_INPUTS:
                return (
                    <EnterInputsStep
                        e3State={e3State}
                        input1={input1}
                        input2={input2}
                        onInput1Change={setInput1}
                        onInput2Change={setInput2}
                        onSubmit={handleInputSubmit}
                    />
                )

            case WizardStep.ENCRYPT_SUBMIT:
                return (
                    <EncryptSubmitStep
                        inputPublishError={inputPublishError}
                        inputPublishSuccess={inputPublishSuccess}
                        onTryAgain={handleTryAgain}
                    />
                )

            case WizardStep.RESULTS:
                return (
                    <ResultsStep
                        input1={input1}
                        input2={input2}
                        result={result}
                        e3State={e3State}
                        transactionHash={transactionHash}
                        onReset={handleReset}
                    />
                )

            default:
                return null
        }
    }

    // ========================================================================
    // RENDER
    // ========================================================================

    return (
        <div className='relative flex w-full flex-1 items-center justify-center px-6 py-16'>
            <div className='absolute bottom-0 right-0 grid w-full grid-cols-2 gap-2 max-md:opacity-30 md:w-[70vh]'>
                <CircularTiles count={4} />
            </div>
            <div className='relative w-full max-w-2xl space-y-8'>
                <div className="text-center">
                    <h1 className='text-h1 font-bold text-slate-600 mb-4'>Enclave Tutorial</h1>
                    <p className="text-slate-500 max-w-lg mx-auto">
                        Learn how to use Encrypted Execution Environments (E3) for secure computation
                    </p>
                </div>

                {renderStepIndicator()}

                <div className="animate-in fade-in duration-300">
                    {renderStepContent()}
                </div>
            </div>
        </div>
    )
}

export default Wizard 