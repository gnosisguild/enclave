import React, { useState, useEffect, useMemo } from 'react'
import { useAccount } from 'wagmi'
import { ConnectKitButton } from 'connectkit'
import { hexToBytes } from 'viem'

// Components
import CardContent from './components/CardContent'
import EnvironmentError from './components/EnvironmentError'
import Spinner from './components/Spinner'

// SDK and utilities
import { useEnclaveSDK } from '@gnosis-guild/enclave-react'
import {
  encodeBfvParams,
  encodeComputeProviderParams,
  calculateStartWindow,
  decodePlaintextOutput,
  DEFAULT_COMPUTE_PROVIDER_PARAMS,
  DEFAULT_E3_CONFIG,
} from '@gnosis-guild/enclave/sdk'
import { HAS_MISSING_ENV_VARS, MISSING_ENV_VARS, getContractAddresses } from '@/utils/env-config'
import { formatContractError } from '@/utils/error-formatting'

// WebAssembly hook
import { useWebAssemblyHook } from '@/hooks/useWebAssembly'

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
  NumberSquareSix,
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

interface E3State {
  id: bigint | null
  isRequested: boolean
  isCommitteePublished: boolean
  isActivated: boolean
  publicKey: `0x${string}` | null
  expiresAt: bigint | null
  plaintextOutput: string | null
  hasPlaintextOutput: boolean
}

// ============================================================================
// ERROR DISPLAY COMPONENT
// ============================================================================

interface ErrorDisplayProps {
  error: any
  showDetails: boolean
  onToggleDetails: () => void
}

const ErrorDisplay: React.FC<ErrorDisplayProps> = ({ error, showDetails, onToggleDetails }) => {
  if (!error) return null

  const userMessage = formatContractError(error)
  const technicalMessage = error.message || JSON.stringify(error, null, 2)

  return (
    <div className='rounded-lg border border-red-200 bg-red-50 p-4'>
      <p className='mb-2 text-sm text-red-600'>
        <strong>Error:</strong> {userMessage}
      </p>
      {userMessage !== technicalMessage && (
        <button onClick={onToggleDetails} className='text-xs text-red-500 underline hover:text-red-700'>
          {showDetails ? 'Hide Details' : 'Show Technical Details'}
        </button>
      )}
      {showDetails && userMessage !== technicalMessage && (
        <pre className='mt-2 overflow-x-auto rounded border border-red-300 bg-red-100 p-2 text-xs text-red-800'>{technicalMessage}</pre>
      )}
    </div>
  )
}

// ============================================================================
// STEP COMPONENTS
// ============================================================================

const ConnectWalletStep: React.FC = () => (
  <CardContent>
    <div className='space-y-6 text-center'>
      <div className='flex justify-center'>
        <Wallet size={48} className='text-enclave-500' />
      </div>
      <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 1: Connect Your Wallet</p>
      <div className='space-y-4'>
        <h3 className='text-lg font-semibold text-slate-700'>Welcome to Enclave</h3>
        <p className='leading-relaxed text-slate-600'>
          Enclave is a protocol for Encrypted Execution Environments (E3) that enables secure computations on private data using fully
          homomorphic encryption (FHE), zero-knowledge proofs, and distributed key cryptography. Connect your wallet to experience
          privacy-preserving computation.
        </p>
        <div className='rounded-lg border border-enclave-200 bg-enclave-50 p-4'>
          <p className='text-sm text-slate-600'>
            <strong>How it works:</strong> You'll request an E3 computation → Ciphernode committee is selected → Committee publishes shared
            public key → You encrypt and submit inputs → Secure computation executes → Only verified outputs are decrypted by the committee.
          </p>
        </div>
      </div>
      <div className='flex justify-center pt-4'>
        <ConnectKitButton />
      </div>
    </div>
  </CardContent>
)

interface RequestComputationStepProps {
  e3State: E3State
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
  onRequestComputation,
}) => (
  <CardContent>
    <div className='space-y-6 text-center'>
      <div className='flex justify-center'>
        <Calculator size={48} className='text-enclave-400' />
      </div>
      <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 2: Request Computation</p>
      <div className='space-y-4'>
        <h3 className='text-lg font-semibold text-slate-700'>Request Encrypted Execution Environment</h3>
        <p className='leading-relaxed text-slate-600'>
          Request an E3 computation from Enclave's decentralized network. This initiates the selection of a Ciphernode Committee through
          cryptographic sortition, who will generate shared keys for securing your computation without any single point of trust.
        </p>
        <div className='rounded-lg border border-blue-200 bg-blue-50 p-4'>
          <p className='text-sm text-slate-600'>
            <strong>Process:</strong> Request E3 → Committee Selection via Sortition → Key Generation → Ready for Activation
          </p>
        </div>

        {/* E3 State Progress */}
        {e3State.id !== null && (
          <div className='space-y-3'>
            <div className='rounded-lg border border-green-200 bg-green-50 p-4'>
              <p className='text-sm text-slate-600'>
                <strong>✅ E3 ID:</strong> {String(e3State.id)}
                <br />
                <strong>Status:</strong> Computation requested
              </p>
            </div>

            {e3State.isCommitteePublished && e3State.publicKey ? (
              <div className='rounded-lg border border-enclave-200 bg-enclave-50 p-4'>
                <p className='text-sm text-slate-600'>
                  <strong>🔑 Committee Published Public Key!</strong>
                  <br />
                  <strong>Public Key:</strong> {e3State.publicKey.slice(0, 20)}...{e3State.publicKey.slice(-10)}
                  <br />
                  Ready to activate E3 environment.
                </p>
              </div>
            ) : (
              <div className='rounded-lg border border-yellow-200 bg-yellow-50 p-4'>
                <div className='flex flex-col items-center space-x-2'>
                  <Spinner size={20} />
                  <p className='text-sm text-slate-600'>
                    <strong>⏳ Waiting for committee to publish public key...</strong>
                    <br />
                    The computation committee is being selected and will provide the public key shortly.
                  </p>
                </div>
              </div>
            )}
          </div>
        )}

        {error && <ErrorDisplay error={error} showDetails={false} onToggleDetails={() => { }} />}

        {isSuccess && transactionHash && (
          <div className='rounded-lg border border-green-200 bg-green-50 p-4'>
            <p className='text-sm text-green-600'>
              <strong>✅ Transaction Successful!</strong>
              <br />
              Hash: {transactionHash.slice(0, 10)}...{transactionHash.slice(-8)}
            </p>
          </div>
        )}
      </div>

      {isRequesting && (
        <div className='mb-4 flex justify-center'>
          <Spinner />
        </div>
      )}

      <button
        onClick={onRequestComputation}
        disabled={isRequesting || e3State.isRequested}
        className='w-full rounded-lg bg-enclave-400 px-6 py-3 font-semibold text-slate-800 transition-all duration-200 hover:bg-enclave-300 hover:shadow-md disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-500'
      >
        {isRequesting
          ? 'Submitting to Blockchain...'
          : e3State.isRequested
            ? e3State.isCommitteePublished
              ? 'Committee Ready - Proceeding to Activation!'
              : 'Waiting for Committee...'
            : 'Request E3 Computation (0.001 ETH)'}
      </button>
    </div>
  </CardContent>
)

interface ActivateE3StepProps {
  e3State: E3State
  isRequesting: boolean
  transactionHash: string | undefined
  error: any
  isSuccess: boolean
  onActivateE3: () => Promise<void>
}

const ActivateE3Step: React.FC<ActivateE3StepProps> = ({ e3State, isRequesting, transactionHash, error, isSuccess, onActivateE3 }) => (
  <CardContent>
    <div className='space-y-6 text-center'>
      <div className='flex justify-center'>
        <Lock size={48} className='text-enclave-400' />
      </div>
      <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 3: Activate E3</p>
      <div className='space-y-4'>
        <h3 className='text-lg font-semibold text-slate-700'>Activate Encrypted Execution Environment</h3>
        <p className='leading-relaxed text-slate-600'>
          Activate the E3 using the Ciphernode Committee's shared public key. This distributed key ensures no single node can decrypt your
          inputs or intermediate states - only the verified final output can be collectively decrypted by the committee.
        </p>

        {e3State.isActivated && e3State.expiresAt && (
          <div className='rounded-lg border border-enclave-200 bg-enclave-50 p-4'>
            <p className='text-sm text-slate-600'>
              <strong>✅ E3 Environment Activated!</strong>
              <br />
              <strong>Expires At:</strong> {new Date(Number(e3State.expiresAt) * 1000).toLocaleString()}
            </p>
          </div>
        )}

        {error && <ErrorDisplay error={error} showDetails={false} onToggleDetails={() => { }} />}

        {isSuccess && transactionHash && (
          <div className='rounded-lg border border-green-200 bg-green-50 p-4'>
            <p className='text-sm text-green-600'>
              <strong>✅ Transaction Successful!</strong>
              <br />
              Hash: {transactionHash.slice(0, 10)}...{transactionHash.slice(-8)}
            </p>
          </div>
        )}
      </div>

      {isRequesting && (
        <div className='mb-4 flex justify-center'>
          <Spinner />
        </div>
      )}

      <button
        onClick={onActivateE3}
        disabled={isRequesting || e3State.isActivated || !e3State.publicKey}
        className='w-full rounded-lg bg-enclave-400 px-6 py-3 font-semibold text-slate-800 transition-all duration-200 hover:bg-enclave-300 hover:shadow-md disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-500'
      >
        {isRequesting
          ? 'Activating...'
          : e3State.isActivated
            ? 'E3 Activated - Ready for Input!'
            : !e3State.publicKey
              ? 'Waiting for Committee Key...'
              : 'Activate E3 Environment'}
      </button>
    </div>
  </CardContent>
)

interface EnterInputsStepProps {
  e3State: E3State
  input1: string
  input2: string
  onInput1Change: (value: string) => void
  onInput2Change: (value: string) => void
  onSubmit: (e: React.FormEvent) => void
}

const EnterInputsStep: React.FC<EnterInputsStepProps> = ({ e3State, input1, input2, onInput1Change, onInput2Change, onSubmit }) => (
  <CardContent>
    <form onSubmit={onSubmit} className='space-y-6 text-center'>
      <div className='flex justify-center'>
        <NumberSquareOne size={48} className='text-enclave-400' />
      </div>
      <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 4: Enter Your Numbers</p>
      <div className='space-y-4'>
        <h3 className='text-lg font-semibold text-slate-700'>Homomorphic Encrypted Computation</h3>
        <p className='leading-relaxed text-slate-600'>
          Enter two numbers for a privacy-preserving addition using fully homomorphic encryption (FHE). Your inputs will be encrypted
          locally and remain encrypted throughout the entire computation process, with only the final result being decrypted.
        </p>
        <div className='rounded-lg border border-blue-200 bg-blue-50 p-4'>
          <p className='text-sm text-slate-600'>
            <strong>Privacy Guarantee:</strong> FHE allows computation on encrypted data. Your numbers remain private throughout the process
            - inputs, intermediate states, and execution are all encrypted.
          </p>
        </div>

        <div className='space-y-4'>
          <div>
            <label htmlFor='input1' className='mb-2 block text-sm font-medium text-slate-700'>
              First Number
            </label>
            <input
              id='input1'
              type='number'
              value={input1}
              onChange={(e) => onInput1Change(e.target.value)}
              className='w-full rounded-md border border-slate-300 px-3 py-2 focus:border-transparent focus:outline-none focus:ring-2 focus:ring-enclave-500'
              placeholder='Enter first number'
              required
            />
          </div>
          <div>
            <label htmlFor='input2' className='mb-2 block text-sm font-medium text-slate-700'>
              Second Number
            </label>
            <input
              id='input2'
              type='number'
              value={input2}
              onChange={(e) => onInput2Change(e.target.value)}
              className='w-full rounded-md border border-slate-300 px-3 py-2 focus:border-transparent focus:outline-none focus:ring-2 focus:ring-enclave-500'
              placeholder='Enter second number'
              required
            />
          </div>
        </div>

        {input1 && input2 && (
          <div className='rounded-lg border border-enclave-200 bg-enclave-50 p-4'>
            <p className='text-sm text-slate-600'>
              <strong>Ready to compute:</strong> {input1} + {input2} = ?
            </p>
          </div>
        )}
      </div>

      <button
        type='submit'
        disabled={!input1 || !input2 || !e3State.isActivated}
        className='w-full rounded-lg bg-enclave-400 px-6 py-3 font-semibold text-slate-800 transition-all duration-200 hover:bg-enclave-300 hover:shadow-md disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-500'
      >
        {!e3State.isActivated ? 'E3 Not Activated Yet' : !input1 || !input2 ? 'Enter Both Numbers' : 'Proceed to Encryption'}
      </button>
    </form>
  </CardContent>
)

interface EncryptSubmitStepProps {
  inputPublishError: string | null
  inputPublishSuccess: boolean
  showErrorDetails: boolean
  onToggleErrorDetails: () => void
  onTryAgain: () => void
}

const EncryptSubmitStep: React.FC<EncryptSubmitStepProps> = ({
  inputPublishError,
  inputPublishSuccess,
  showErrorDetails,
  onToggleErrorDetails,
  onTryAgain,
}) => (
  <CardContent>
    <div className='space-y-6 text-center'>
      <div className='flex justify-center'>
        <Lock size={48} className='text-enclave-400' />
      </div>
      <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 5: Encrypting & Submitting</p>
      <div className='space-y-4'>
        <h3 className='text-lg font-semibold text-slate-700'>Secure Process Execution</h3>

        {!inputPublishError && !inputPublishSuccess && (
          <div className='space-y-4'>
            <div className='flex justify-center'>
              <Spinner size={40} />
            </div>
            <p className='text-slate-600'>
              Your inputs are being encrypted to the committee's public key and submitted to the E3. The Compute Provider will execute the
              FHE computation over your encrypted data...
            </p>
            <div className='rounded-lg border border-blue-200 bg-blue-50 p-4'>
              <p className='text-sm text-slate-600'>
                <strong>Process:</strong> Encrypt to Key → Submit to E3 → FHE Computation → Ciphertext Output
              </p>
            </div>
          </div>
        )}

        {inputPublishError && (
          <div className='space-y-4'>
            <ErrorDisplay error={inputPublishError} showDetails={showErrorDetails} onToggleDetails={onToggleErrorDetails} />
            <button
              onClick={onTryAgain}
              className='w-full rounded-lg bg-red-500 px-6 py-3 font-semibold text-white transition-all duration-200 hover:bg-red-600'
            >
              Try Again
            </button>
          </div>
        )}

        {inputPublishSuccess && (
          <div className='space-y-4'>
            <div className='flex justify-center'>
              <CheckCircle size={48} className='text-green-500' />
            </div>
            <div className='rounded-lg border border-green-200 bg-green-50 p-4'>
              <p className='text-sm text-green-600'>
                <strong>✅ Inputs Successfully Submitted!</strong>
                <br />
                Your encrypted inputs have been published to the E3. The Compute Provider is executing the FHE computation and will publish
                the ciphertext output for committee decryption.
              </p>
            </div>
            <div className='rounded-lg border border-yellow-200 bg-yellow-50 p-4'>
              <div className='mb-2 flex justify-center'>
                <Spinner size={20} />
              </div>
              <p className='text-sm text-slate-600'>
                <strong>Computing...</strong> Waiting for the Ciphernode Committee to collectively decrypt the verified output.
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  </CardContent>
)

interface ResultsStepProps {
  input1: string
  input2: string
  result: number | null
  e3State: E3State
  transactionHash: string | undefined
  onReset: () => void
}

const ResultsStep: React.FC<ResultsStepProps> = ({ input1, input2, result, e3State, transactionHash, onReset }) => (
  <CardContent>
    <div className='space-y-6 text-center'>
      <div className='flex justify-center'>
        <CheckCircle size={48} className='text-green-500' />
      </div>
      <p className='text-base font-extrabold uppercase text-slate-600/50'>Step 6: Results</p>
      <div className='space-y-4'>
        <h3 className='text-lg font-semibold text-slate-700'>Computation Complete!</h3>

        <div className='rounded-lg border border-green-200 bg-green-50 p-6'>
          <div className='space-y-3'>
            <p className='text-lg font-semibold text-slate-700'>
              <strong>Your Encrypted Computation:</strong>
            </p>
            <p className='text-2xl font-bold text-green-700'>
              {input1} + {input2} = {result !== null ? result : 'Computing...'}
            </p>
            {result !== null && <p className='text-sm text-slate-600'>✅ Computed securely using FHE with distributed key decryption!</p>}
          </div>
        </div>

        <div className='grid grid-cols-1 gap-3 text-left'>
          <div className='rounded-lg border border-slate-200 bg-slate-50 p-4'>
            <p className='text-sm text-slate-600'>
              <strong>E3 ID:</strong> {String(e3State.id)}
            </p>
          </div>
          {transactionHash && (
            <div className='rounded-lg border border-slate-200 bg-slate-50 p-4'>
              <p className='text-sm text-slate-600'>
                <strong>Transaction:</strong> {transactionHash.slice(0, 10)}...{transactionHash.slice(-8)}
              </p>
            </div>
          )}
          {e3State.plaintextOutput && (
            <div className='rounded-lg border border-slate-200 bg-slate-50 p-4'>
              <p className='text-sm text-slate-600'>
                <strong>Raw Output:</strong> {e3State.plaintextOutput.slice(0, 20)}...
              </p>
            </div>
          )}
        </div>

        <div className='rounded-lg border border-blue-200 bg-blue-50 p-4'>
          <p className='text-sm text-slate-600'>
            <strong>🔒 Cryptographic Guarantees:</strong> Your inputs remained encrypted throughout the entire process. The Ciphernode
            Committee used distributed key cryptography to decrypt only the verified output, ensuring data privacy, data integrity, and
            correct execution.
          </p>
        </div>
      </div>

      <button
        onClick={onReset}
        className='w-full rounded-lg bg-enclave-400 px-6 py-3 font-semibold text-slate-800 transition-all duration-200 hover:bg-enclave-300 hover:shadow-md'
      >
        Start New Computation
      </button>
    </div>
  </CardContent>
)

// ============================================================================
// MAIN WIZARD COMPONENT
// ============================================================================

const WizardSDK: React.FC = () => {
  const { isConnected } = useAccount()
  const { isLoaded: isWasmLoaded, encryptInput } = useWebAssemblyHook()

  if (HAS_MISSING_ENV_VARS) {
    return <EnvironmentError missingVars={MISSING_ENV_VARS} />
  }
  const contracts = getContractAddresses()
  const sdkConfig = useMemo(
    () => ({
      autoConnect: true,
      contracts: {
        enclave: contracts.enclave,
        ciphernodeRegistry: contracts.ciphernodeRegistry,
      },
    }),
    [contracts.enclave, contracts.ciphernodeRegistry],
  )

  const {
    isInitialized,
    error: sdkError,
    requestE3,
    activateE3,
    publishInput,
    onEnclaveEvent,
    off,
    EnclaveEventType,
    RegistryEventType,
  } = useEnclaveSDK(sdkConfig)

  // Component state
  const [currentStep, setCurrentStep] = useState<WizardStep>(WizardStep.CONNECT_WALLET)
  const [input1, setInput1] = useState('')
  const [input2, setInput2] = useState('')
  const [lastTransactionHash, setLastTransactionHash] = useState<string | undefined>()
  const [inputPublishError, setInputPublishError] = useState<string | null>(null)
  const [inputPublishSuccess, setInputPublishSuccess] = useState(false)
  const [requestError, setRequestError] = useState<any>(null)
  const [showErrorDetails, setShowErrorDetails] = useState(false)
  const [isRequesting, setIsRequesting] = useState(false)
  const [requestSuccess, setRequestSuccess] = useState(false)
  const [result, setResult] = useState<number | null>(null)

  // E3 state tracking
  const [e3State, setE3State] = useState<E3State>({
    id: null,
    isRequested: false,
    isCommitteePublished: false,
    isActivated: false,
    publicKey: null,
    expiresAt: null,
    plaintextOutput: null,
    hasPlaintextOutput: false,
  })

  // Set up event listeners
  useEffect(() => {
    if (!isInitialized) return

    const handleE3Requested = (event: any) => {
      const e3Id = event.data.e3Id
      setE3State((prev) => ({
        ...prev,
        id: e3Id,
        isRequested: true,
      }))
    }

    const handleCommitteePublished = (event: any) => {
      const { e3Id, publicKey } = event.data

      // I added a 2 second delay to show the waiting state, its too fast on anvil
      setTimeout(() => {
        setE3State((prev) => {
          if (prev.id !== null && e3Id === prev.id) {
            return {
              ...prev,
              isCommitteePublished: true,
              publicKey: publicKey as `0x${string}`,
            }
          }
          return prev
        })
      }, 2000)
    }

    const handleE3Activated = (event: any) => {
      const { e3Id, expiration } = event.data
      setE3State((prev) => {
        if (prev.id !== null && e3Id === prev.id) {
          return {
            ...prev,
            isActivated: true,
            expiresAt: expiration || null,
          }
        }
        return prev
      })
    }

    const handlePlaintextOutput = (event: any) => {
      const { e3Id, plaintextOutput } = event.data
      setE3State((prev) => {
        if (prev.id !== null && e3Id === prev.id) {
          const decodedResult = decodePlaintextOutput(plaintextOutput)
          setResult(decodedResult)
          return {
            ...prev,
            plaintextOutput: plaintextOutput as string,
            hasPlaintextOutput: true,
          }
        }
        return prev
      })
    }

    // Set up event listeners
    onEnclaveEvent(EnclaveEventType.E3_REQUESTED, handleE3Requested)
    onEnclaveEvent(RegistryEventType.COMMITTEE_PUBLISHED, handleCommitteePublished)
    onEnclaveEvent(EnclaveEventType.E3_ACTIVATED, handleE3Activated)
    onEnclaveEvent(EnclaveEventType.PLAINTEXT_OUTPUT_PUBLISHED, handlePlaintextOutput)

    // Cleanup
    return () => {
      off(EnclaveEventType.E3_REQUESTED, handleE3Requested)
      off(RegistryEventType.COMMITTEE_PUBLISHED, handleCommitteePublished)
      off(EnclaveEventType.E3_ACTIVATED, handleE3Activated)
      off(EnclaveEventType.PLAINTEXT_OUTPUT_PUBLISHED, handlePlaintextOutput)
    }
  }, [isInitialized, onEnclaveEvent, off, EnclaveEventType, RegistryEventType])
  console.log({ currentStep })
  // Auto-advance steps based on state
  useEffect(() => {
    if (!isConnected && currentStep > WizardStep.CONNECT_WALLET) {
      setCurrentStep(WizardStep.CONNECT_WALLET)
    } else if (isConnected && isInitialized && currentStep === WizardStep.CONNECT_WALLET) {
      setCurrentStep(WizardStep.REQUEST_COMPUTATION)
    } else if (e3State.isCommitteePublished && currentStep === WizardStep.REQUEST_COMPUTATION) {
      setCurrentStep(WizardStep.ACTIVATE_E3)
    } else if (e3State.isActivated && currentStep === WizardStep.ACTIVATE_E3) {
      setCurrentStep(WizardStep.ENTER_INPUTS)
    } else if (e3State.hasPlaintextOutput && currentStep < WizardStep.RESULTS) {
      setCurrentStep(WizardStep.RESULTS)
    }
  }, [isConnected, isInitialized, currentStep, e3State])

  const handleRequestComputation = async () => {
    setIsRequesting(true)
    setRequestError(null)
    setRequestSuccess(false)

    // Reset E3 state
    setE3State({
      id: null,
      isRequested: false,
      isCommitteePublished: false,
      isActivated: false,
      publicKey: null,
      expiresAt: null,
      plaintextOutput: null,
      hasPlaintextOutput: false,
    })

    try {
      const threshold: [number, number] = [DEFAULT_E3_CONFIG.threshold_min, DEFAULT_E3_CONFIG.threshold_max]
      const startWindow = calculateStartWindow(60) // 1 minute
      const duration = BigInt(60) // 1 minute
      const e3ProgramParams = encodeBfvParams()
      const computeProviderParams = encodeComputeProviderParams(DEFAULT_COMPUTE_PROVIDER_PARAMS)

      const hash = await requestE3({
        filter: contracts.filterRegistry,
        threshold,
        startWindow,
        duration,
        e3Program: contracts.e3Program,
        e3ProgramParams,
        computeProviderParams,
        value: BigInt('1000000000000000'), // 0.001 ETH
      })

      setLastTransactionHash(hash)
      setRequestSuccess(true)
    } catch (error) {
      setRequestError(error)
      console.error('Error requesting computation:', error)
    } finally {
      setIsRequesting(false)
    }
  }

  const handleActivateE3 = async () => {
    if (e3State.id === null || e3State.publicKey === null) return

    setIsRequesting(true)
    setRequestError(null)

    try {
      const hash = await activateE3(e3State.id, e3State.publicKey)
      setLastTransactionHash(hash)
      setRequestSuccess(true)
    } catch (error) {
      setRequestError(error)
      console.error('Error activating E3:', error)
    } finally {
      setIsRequesting(false)
    }
  }

  const handleInputSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!input1 || !input2 || e3State.publicKey === null || e3State.id === null) return

    setCurrentStep(WizardStep.ENCRYPT_SUBMIT)
    setInputPublishError(null)
    setInputPublishSuccess(false)

    try {
      // Parse inputs
      const num1 = BigInt(input1)
      const num2 = BigInt(input2)

      // Convert hex public key to bytes
      const publicKeyBytes = hexToBytes(e3State.publicKey)

      // Encrypt both inputs
      const encryptedInput1 = await encryptInput(num1, publicKeyBytes)
      const encryptedInput2 = await encryptInput(num2, publicKeyBytes)

      if (!encryptedInput1 || !encryptedInput2) {
        throw new Error('Failed to encrypt inputs')
      }

      // Publish first input
      await publishInput(e3State.id, `0x${Array.from(encryptedInput1, (b) => b.toString(16).padStart(2, '0')).join('')}` as `0x${string}`)

      // Publish second input
      const hash2 = await publishInput(
        e3State.id,
        `0x${Array.from(encryptedInput2, (b) => b.toString(16).padStart(2, '0')).join('')}` as `0x${string}`,
      )

      setLastTransactionHash(hash2)
      setInputPublishSuccess(true)
    } catch (error) {
      setInputPublishError(error instanceof Error ? error.message : 'Failed to encrypt and publish inputs')
      console.error('Error encrypting/publishing inputs:', error)
    }
  }

  const handleReset = () => {
    setCurrentStep(WizardStep.CONNECT_WALLET)
    setInput1('')
    setInput2('')
    setLastTransactionHash(undefined)
    setInputPublishError(null)
    setInputPublishSuccess(false)
    setRequestError(null)
    setIsRequesting(false)
    setRequestSuccess(false)
    setResult(null)
    setE3State({
      id: null,
      isRequested: false,
      isCommitteePublished: false,
      isActivated: false,
      publicKey: null,
      expiresAt: null,
      plaintextOutput: null,
      hasPlaintextOutput: false,
    })
  }

  const handleTryAgain = () => {
    setCurrentStep(WizardStep.ENTER_INPUTS)
    setInputPublishError(null)
    setInputPublishSuccess(false)
  }

  const getStepIcon = (step: WizardStep) => {
    const iconProps = { size: 24, className: currentStep >= step ? 'text-enclave-500' : 'text-slate-400' }
    switch (step) {
      case WizardStep.CONNECT_WALLET:
        return <NumberSquareOne {...iconProps} />
      case WizardStep.REQUEST_COMPUTATION:
        return <NumberSquareTwo {...iconProps} />
      case WizardStep.ACTIVATE_E3:
        return <NumberSquareThree {...iconProps} />
      case WizardStep.ENTER_INPUTS:
        return <NumberSquareFour {...iconProps} />
      case WizardStep.ENCRYPT_SUBMIT:
        return <NumberSquareFive {...iconProps} />
      case WizardStep.RESULTS:
        return <NumberSquareSix {...iconProps} />
    }
  }

  const renderStepIndicator = () => (
    <div className='mb-8 flex justify-center'>
      <div className='flex items-center space-x-2'>
        {[1, 2, 3, 4, 5, 6].map((step) => (
          <div key={step} className='flex items-center'>
            <div
              className={`flex h-10 w-10 items-center justify-center rounded-full border-2 transition-all duration-200 ${currentStep >= step ? 'border-enclave-400 bg-enclave-100 text-enclave-600' : 'border-slate-300 bg-slate-100 text-slate-400'
                }`}
            >
              {getStepIcon(step as WizardStep)}
            </div>
            {step < 6 && (
              <div className={`mx-2 h-0.5 w-8 transition-all duration-200 ${currentStep > step ? 'bg-enclave-400' : 'bg-slate-300'}`} />
            )}
          </div>
        ))}
      </div>
    </div>
  )

  const renderStepContent = () => {
    switch (currentStep) {
      case WizardStep.CONNECT_WALLET:
        return <ConnectWalletStep />
      case WizardStep.REQUEST_COMPUTATION:
        return (
          <RequestComputationStep
            e3State={e3State}
            isRequesting={isRequesting}
            transactionHash={lastTransactionHash}
            error={requestError}
            isSuccess={requestSuccess}
            onRequestComputation={handleRequestComputation}
          />
        )
      case WizardStep.ACTIVATE_E3:
        return (
          <ActivateE3Step
            e3State={e3State}
            isRequesting={isRequesting}
            transactionHash={lastTransactionHash}
            error={requestError}
            isSuccess={requestSuccess}
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
            showErrorDetails={showErrorDetails}
            onToggleErrorDetails={() => setShowErrorDetails(!showErrorDetails)}
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
            transactionHash={lastTransactionHash}
            onReset={handleReset}
          />
        )
      default:
        return null
    }
  }

  if (sdkError) {
    return (
      <div className='min-h-screen bg-gray-100 px-4 py-12 sm:px-6 lg:px-8'>
        <div className='mx-auto max-w-md'>
          <div className='rounded-md border border-red-200 bg-red-50 p-4'>
            <div className='flex'>
              <div className='ml-3'>
                <h3 className='text-sm font-medium text-red-800'>SDK Error</h3>
                <div className='mt-2 text-sm text-red-700'>{sdkError}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className='min-h-screen bg-gradient-to-br from-slate-50 to-slate-100 text-slate-900'>
      <div className='container mx-auto px-4 py-8'>
        <div className='mb-8 text-center'>
          <h1 className='mb-2 text-4xl font-bold text-slate-800'>Enclave E3</h1>
          <p className='text-lg text-slate-600'>Confidential computation with Enclave Encrypted Execution Environments</p>
        </div>

        {renderStepIndicator()}

        <div className='mx-auto max-w-2xl'>{renderStepContent()}</div>

        {!isWasmLoaded && (
          <div className='fixed bottom-4 right-4 rounded-lg border border-yellow-400 bg-yellow-100 p-3'>
            <p className='text-sm text-yellow-800'>Loading encryption module...</p>
          </div>
        )}
      </div>
    </div>
  )
}

export default WizardSDK
