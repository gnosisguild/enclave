import React, { useState, useEffect } from 'react'
import { useAccount } from 'wagmi'
import { ConnectKitButton } from 'connectkit'
import { hexToBytes } from 'viem'
import CardContent from '@/components/Cards/CardContent'
import CircularTiles from '@/components/CircularTiles'
import LoadingAnimation from '@/components/LoadingAnimation'
import EnvironmentError from '@/components/EnvironmentError'
import { useEnclaveContract } from '@/hooks/enclave/useEnclaveContract'
import { HAS_MISSING_ENV_VARS, MISSING_ENV_VARS } from '@/config/Enclave.abi'
import { WalletIcon, CalculatorIcon, LockIcon, CheckCircleIcon, NumberSquareOneIcon, NumberSquareTwoIcon, NumberSquareThreeIcon, NumberSquareFourIcon, NumberSquareFiveIcon, NumberSquareTwo } from '@phosphor-icons/react'
import { useWebAssemblyHook } from '@/hooks/wasm/useWebAssembly';

enum WizardStep {
    CONNECT_WALLET = 1,
    REQUEST_COMPUTATION = 2,
    ACTIVATE_E3 = 3,
    ENTER_INPUTS = 4,
    ENCRYPT_SUBMIT = 5,
    RESULTS = 6,
}

const Wizard: React.FC = () => {
    if (HAS_MISSING_ENV_VARS) {
        return <EnvironmentError missingVars={MISSING_ENV_VARS} />
    }

    const [currentStep, setCurrentStep] = useState<WizardStep>(WizardStep.CONNECT_WALLET)
    const [input1, setInput1] = useState<string>('')
    const [input2, setInput2] = useState<string>('')
    const [isLoading, setIsLoading] = useState<boolean>(false)
    const [encryptedInputs, setEncryptedInputs] = useState<{ input1: Uint8Array; input2: Uint8Array } | undefined>(undefined)
    const [result, setResult] = useState<number | null>(null)
    const { encryptInput } = useWebAssemblyHook()

    const { isConnected } = useAccount()

    // Enclave contract integration
    const { requestComputation, activateE3, publishInput, e3State, isRequesting, isSuccess, error, transactionHash } = useEnclaveContract()

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

    // Handle E3 lifecycle progression
    useEffect(() => {
        if (e3State.isCommitteePublished && e3State.publicKey && currentStep === WizardStep.REQUEST_COMPUTATION) {
            console.log('🔄 Committee ready - advancing to ACTIVATE_E3 step')
            setCurrentStep(WizardStep.ACTIVATE_E3)
        }
    }, [e3State.isCommitteePublished, e3State.publicKey, currentStep])

    // Auto-advance to input step when E3 is activated
    useEffect(() => {
        if (e3State.isActivated && currentStep === WizardStep.ACTIVATE_E3) {
            console.log('🔄 E3 activated - advancing to ENTER_INPUTS step')
            setCurrentStep(WizardStep.ENTER_INPUTS)
        }
    }, [e3State.isActivated, currentStep])

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

        try {
            await new Promise(resolve => setTimeout(resolve, 1500))

            const publicKeyBytes = hexToBytes(e3State.publicKey as `0x${string}`)
            console.log('publicKeyBytes', publicKeyBytes)
            console.log('input1', input1)
            console.log('input2', input2)
            const encryptedInput1 = await encryptInput(BigInt(input1), publicKeyBytes)
            console.log('encryptedInput1', encryptedInput1)
            const encryptedInput2 = await encryptInput(BigInt(input2), publicKeyBytes)
            console.log('encryptedInput2', encryptedInput2)

            if (!encryptedInput1 || !encryptedInput2) {
                throw new Error('Encryption failed')
            }

            setEncryptedInputs({
                input1: encryptedInput1,
                input2: encryptedInput2
            })

            // Publish the encrypted inputs to the Enclave contract
            await publishInput(encryptedInput1)
            await new Promise(resolve => setTimeout(resolve, 1000))
            await publishInput(encryptedInput2)

            // Simulate computation result - sum of the inputs
            const sum = parseInt(input1) + parseInt(input2)
            setResult(sum)

            // Another loading simulation for "computation"
            await new Promise(resolve => setTimeout(resolve, 1000))

            setIsLoading(false)
            setCurrentStep(WizardStep.RESULTS)
        } catch (error) {
            console.error('Encryption or publishing failed:', error)
            setIsLoading(false)
        }
    }

    const handleReset = () => {
        setCurrentStep(WizardStep.REQUEST_COMPUTATION)
        setInput1('')
        setInput2('')
        setEncryptedInputs(undefined)
        setResult(null)
    }

    const getStepIcon = (step: WizardStep) => {
        const iconProps = { size: 20, weight: 'bold' as const }
        switch (step) {
            case WizardStep.CONNECT_WALLET: return <NumberSquareOneIcon {...iconProps} />
            case WizardStep.REQUEST_COMPUTATION: return <NumberSquareTwoIcon {...iconProps} />
            case WizardStep.ACTIVATE_E3: return <NumberSquareThreeIcon {...iconProps} />
            case WizardStep.ENTER_INPUTS: return <NumberSquareFourIcon {...iconProps} />
            case WizardStep.ENCRYPT_SUBMIT: return <NumberSquareFiveIcon {...iconProps} />
            case WizardStep.RESULTS: return <NumberSquareFiveIcon {...iconProps} />
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

    const renderStepContent = () => {
        switch (currentStep) {
            case WizardStep.CONNECT_WALLET:
                return (
                    <CardContent>
                        <div className='space-y-6 text-center'>
                            <div className="flex justify-center">
                                <WalletIcon size={48} className="text-lime-400" />
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

            case WizardStep.REQUEST_COMPUTATION:
                return (
                    <CardContent>
                        <div className='space-y-6 text-center'>
                            <div className="flex justify-center">
                                <CalculatorIcon size={48} className="text-lime-400" />
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

                                {transactionHash && (
                                    <div className="bg-slate-50 border border-slate-200 rounded-lg p-4">
                                        <p className="text-sm text-slate-600">
                                            <strong>Transaction:</strong> {transactionHash.slice(0, 10)}...{transactionHash.slice(-8)}
                                        </p>
                                    </div>
                                )}

                                {error && (
                                    <div className="bg-red-50 border border-red-200 rounded-lg p-4">
                                        <p className="text-sm text-red-600">
                                            <strong>Error:</strong> {error.message}
                                        </p>
                                    </div>
                                )}
                            </div>

                            <button
                                onClick={handleRequestComputation}
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

            case WizardStep.ACTIVATE_E3:
                return (
                    <CardContent>
                        <div className='space-y-6 text-center'>
                            <div className="flex justify-center">
                                <LockIcon size={48} className="text-lime-400" />
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
                            </div>

                            <button
                                onClick={handleActivateE3}
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

            case WizardStep.ENTER_INPUTS:
                return (
                    <CardContent>
                        <div className='space-y-6'>
                            <div className="text-center">
                                <div className="flex justify-center mb-4">
                                    <LockIcon size={48} className="text-lime-400" />
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

                            <form onSubmit={handleInputSubmit} className='space-y-6'>
                                <div className='space-y-2'>
                                    <label htmlFor='input1' className='block text-sm font-medium text-slate-700'>
                                        First Number
                                    </label>
                                    <input
                                        type='number'
                                        id='input1'
                                        value={input1}
                                        onChange={(e) => setInput1(e.target.value)}
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
                                        onChange={(e) => setInput2(e.target.value)}
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

            case WizardStep.ENCRYPT_SUBMIT:
                return (
                    <CardContent>
                        <div className='space-y-6 text-center'>
                            <div className="flex justify-center">
                                <LockIcon size={48} className="text-lime-400 animate-pulse" />
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
                            </div>
                            <div className="pt-4">
                                <LoadingAnimation isLoading={true} />
                                <p className="text-sm text-slate-500 mt-4">This may take a moment...</p>
                            </div>
                        </div>
                    </CardContent>
                )

            case WizardStep.RESULTS:
                return (
                    <CardContent>
                        <div className='space-y-6 text-center'>
                            <div className="flex justify-center">
                                <CheckCircleIcon size={48} className="text-green-500" />
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
                                    onClick={handleReset}
                                    className='w-full rounded-lg bg-slate-600 px-6 py-3 font-semibold text-white transition-all duration-200 hover:bg-slate-500 hover:shadow-md hover:scale-[1.02]'
                                >
                                    Try Another Computation
                                </button>
                            </div>
                        </div>
                    </CardContent>
                )

            default:
                return null
        }
    }

    return (
        <div className='relative flex w-full flex-1 items-center justify-center px-6 py-16'>
            <div className='absolute bottom-0 right-0 grid w-full grid-cols-2 gap-2 max-md:opacity-30 md:w-[70vh]'>
                <CircularTiles count={4} />
            </div>
            <div className='relative w-full max-w-2xl space-y-8'>
                <div className="text-center">
                    <h1 className='text-h1 font-bold text-slate-600 mb-4'>Encrypted Computation Wizard</h1>
                    <p className="text-slate-500 max-w-lg mx-auto">
                        Experience homomorphic encryption in action with CRISP's secure computation environment
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

export default Wizard; 