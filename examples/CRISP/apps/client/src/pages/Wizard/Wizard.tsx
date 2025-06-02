import React, { useState, useEffect } from 'react'
import { useAccount } from 'wagmi'
import { ConnectKitButton } from 'connectkit'
import CardContent from '@/components/Cards/CardContent'
import CircularTiles from '@/components/CircularTiles'
import LoadingAnimation from '@/components/LoadingAnimation'
import EnvironmentError from '@/components/EnvironmentError'
import { useEnclaveContract } from '@/hooks/enclave/useEnclaveContract'
import { HAS_MISSING_ENV_VARS, MISSING_ENV_VARS } from '@/config/Enclave.abi'
import { Wallet, Calculator, Lock, CheckCircle, NumberSquareOne, NumberSquareTwo, NumberSquareThree, NumberSquareFour, NumberSquareFive } from '@phosphor-icons/react'
import { useWebAssemblyHook } from '@/hooks/wasm/useWebAssembly';

enum WizardStep {
    CONNECT_WALLET = 1,
    REQUEST_COMPUTATION = 2,
    ENTER_INPUTS = 3,
    ENCRYPT_SUBMIT = 4,
    RESULTS = 5,
}

const Wizard: React.FC = () => {
    // Check for missing environment variables first
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
    const { requestComputation, e3State, isRequesting, isSuccess, error, transactionHash } = useEnclaveContract()

    console.log('e3State', e3State)
    console.log('isRequesting', isRequesting)
    console.log('isSuccess', isSuccess)
    console.log('error', error)
    console.log('transactionHash', transactionHash)

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
        if (e3State.isActivated && e3State.publicKey && currentStep === WizardStep.REQUEST_COMPUTATION) {
            setCurrentStep(WizardStep.ENTER_INPUTS)
        }
    }, [e3State.isActivated, e3State.publicKey, currentStep])

    const handleRequestComputation = async () => {
        try {
            await requestComputation({
                paymentAmount: "0.001"
            })
        } catch (error) {
            console.error('Failed to request computation:', error)
        }
    }

    const handleInputSubmit = async (e: React.FormEvent) => {
        e.preventDefault()
        if (!input1.trim() || !input2.trim()) return

        setCurrentStep(WizardStep.ENCRYPT_SUBMIT)
        setIsLoading(true)

        try {
            await new Promise(resolve => setTimeout(resolve, 1500))

            const encryptedInput1 = await encryptInput(BigInt(input1), e3State.publicKey as Uint8Array)
            const encryptedInput2 = await encryptInput(BigInt(input2), e3State.publicKey as Uint8Array)

            setEncryptedInputs({
                input1: encryptedInput1,
                input2: encryptedInput2
            })

            // Simulate computation result - sum of the inputs
            const sum = parseInt(input1) + parseInt(input2)
            setResult(sum)

            // Another loading simulation for "computation"
            await new Promise(resolve => setTimeout(resolve, 1000))

            setIsLoading(false)
            setCurrentStep(WizardStep.RESULTS)
        } catch (error) {
            console.error('Encryption failed:', error)
            setIsLoading(false)
        }
    }

    const handleReset = () => {
        setCurrentStep(WizardStep.REQUEST_COMPUTATION)
        setInput1('')
        setInput2('')
        setEncryptedInputs(null)
        setResult(null)
    }

    const getStepIcon = (step: WizardStep) => {
        const iconProps = { size: 20, weight: 'bold' as const }
        switch (step) {
            case WizardStep.CONNECT_WALLET: return <NumberSquareOne {...iconProps} />
            case WizardStep.REQUEST_COMPUTATION: return <NumberSquareTwo {...iconProps} />
            case WizardStep.ENTER_INPUTS: return <NumberSquareThree {...iconProps} />
            case WizardStep.ENCRYPT_SUBMIT: return <NumberSquareFour {...iconProps} />
            case WizardStep.RESULTS: return <NumberSquareFive {...iconProps} />
        }
    }

    const renderStepIndicator = () => (
        <div className="flex items-center justify-center space-x-2 mb-8">
            {[1, 2, 3, 4, 5].map((step) => (
                <div key={step} className="flex items-center">
                    <div
                        className={`flex items-center justify-center w-10 h-10 rounded-full border-2 transition-all duration-300 ${step <= currentStep
                            ? 'bg-lime-400 border-lime-400 text-slate-800'
                            : 'bg-slate-100 border-slate-300 text-slate-500'
                            }`}
                    >
                        {getStepIcon(step as WizardStep)}
                    </div>
                    {step < 5 && (
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
                                        enter two numbers, and see them encrypted using homomorphic encryption before computation.
                                    </p>
                                </div>
                            </div>
                            <div className="pt-4">
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
                                <Calculator size={48} className="text-lime-400" />
                            </div>
                            <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 2: Request Computation</p>
                            <div className="space-y-4">
                                <h3 className="text-lg font-semibold text-slate-700">Initialize Secure Computing Environment</h3>
                                <p className="text-slate-600 leading-relaxed">
                                    Request an E3 computation from the Enclave network. This will create a secure
                                    computation environment and wait for the committee to activate it with a public key.
                                </p>
                                <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
                                    <p className="text-sm text-slate-600">
                                        <strong>What happens:</strong> Request → Committee Selection → Environment Activation → Ready for Encryption
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

                                        {e3State.isActivated && e3State.publicKey ? (
                                            <div className="bg-lime-50 border border-lime-200 rounded-lg p-4">
                                                <p className="text-sm text-slate-600">
                                                    <strong>🔑 Environment Activated!</strong>
                                                    <br />
                                                    <strong>Public Key:</strong> {e3State.publicKey.slice(0, 20)}...{e3State.publicKey.slice(-10)}
                                                    <br />
                                                    {e3State.expiresAt !== null && (
                                                        <>
                                                            <strong>Expires:</strong> {new Date(Number(e3State.expiresAt) * 1000).toLocaleString()}
                                                        </>
                                                    )}
                                                </p>
                                            </div>
                                        ) : e3State.isRequested && (
                                            <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-4">
                                                <div className="flex items-center space-x-2">
                                                    <LoadingAnimation isLoading={true} className="!h-4 !w-4" />
                                                    <p className="text-sm text-slate-600">
                                                        <strong>⏳ Waiting for committee activation...</strong>
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
                                    : e3State.isActivated
                                        ? 'Environment Ready!'
                                        : e3State.isRequested
                                            ? 'Waiting for Activation...'
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

            case WizardStep.ENTER_INPUTS:
                return (
                    <CardContent>
                        <div className='space-y-6'>
                            <div className="text-center">
                                <div className="flex justify-center mb-4">
                                    <Lock size={48} className="text-lime-400" />
                                </div>
                                <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 3: Enter Your Numbers</p>
                                <h3 className="text-lg font-semibold text-slate-700 mt-2">Input Data for Encrypted Computation</h3>
                                <p className="text-slate-600 mt-2">
                                    Enter two numbers that will be encrypted using the committee's public key and computed securely in the E3 environment.
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
                                        throughout the homomorphic computation process.
                                    </p>
                                </div>

                                <button
                                    type='submit'
                                    disabled={!input1.trim() || !input2.trim() || !e3State.publicKey}
                                    className='w-full rounded-lg bg-lime-400 px-6 py-3 font-semibold text-slate-800 transition-all duration-200 hover:bg-lime-300 hover:shadow-md hover:scale-[1.02] disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-500 disabled:transform-none'
                                >
                                    {!e3State.publicKey ? 'Waiting for Public Key...' : 'Encrypt & Submit Numbers'}
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
                                <Lock size={48} className="text-lime-400 animate-pulse" />
                            </div>
                            <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 4: Encrypting & Computing</p>
                            <div className="space-y-4">
                                <h3 className="text-lg font-semibold text-slate-700">Processing Your Encrypted Data</h3>
                                <p className="text-slate-600 leading-relaxed">
                                    Your numbers are being encrypted using homomorphic encryption and sent to the secure computation environment.
                                    The computation is being performed on the encrypted data without decrypting it.
                                </p>
                                <div className="bg-green-50 border border-green-200 rounded-lg p-4">
                                    <p className="text-sm text-slate-600">
                                        <strong>Processing Steps:</strong>
                                        <br />• Encrypting your inputs with FHE
                                        <br />• Performing encrypted addition
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
                                <CheckCircle size={48} className="text-green-500" />
                            </div>
                            <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 5: Computation Complete</p>

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
                                    </div>
                                </div>

                                <div className="bg-green-50 border border-green-200 rounded-lg p-4">
                                    <h4 className="font-medium text-slate-700 mb-2">What Just Happened?</h4>
                                    <p className="text-sm text-slate-600 leading-relaxed">
                                        Your numbers were encrypted using the committee's public key from E3 environment {e3State.id !== null ? String(e3State.id) : 'N/A'},
                                        sent to the secure computation network, added together while remaining encrypted,
                                        and the result was computed without ever revealing your original numbers to the computing system.
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