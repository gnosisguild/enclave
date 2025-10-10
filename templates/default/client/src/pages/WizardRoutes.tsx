// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { useMemo } from 'react'
import { Routes, Route, Navigate } from 'react-router-dom'
import {
  NumberSquareOneIcon,
  NumberSquareTwoIcon,
  NumberSquareThreeIcon,
  NumberSquareFourIcon,
  NumberSquareFiveIcon,
  NumberSquareSixIcon,
} from '@phosphor-icons/react'

// Step components
import ConnectWallet from './steps/ConnectWallet'
import RequestComputation from './steps/RequestComputation'
import ActivateE3 from './steps/ActivateE3'
import EnterInputs from './steps/EnterInputs'
import EncryptSubmit from './steps/EncryptSubmit'
import Results from './steps/Results'
import EnvironmentError from './components/EnvironmentError'
import SDKErrorDisplay from './components/SDKErrorDisplay'
import StepIndicator from './components/StepIndicator'
import { useWizard, WizardStep } from '../context/WizardContext'
import { MISSING_ENV_VARS } from '@/utils/env-config'

interface StepConfig {
  step: WizardStep
  path: string
  component: React.ComponentType
  icon: React.ComponentType<any>
}

// Steps are defined below as an array of objects.
// Each entry specifies the wizard step, URL path, component, and icon for that step.
const STEPS: StepConfig[] = [
  { step: WizardStep.CONNECT_WALLET, path: '/step1', component: ConnectWallet, icon: NumberSquareOneIcon },
  { step: WizardStep.REQUEST_COMPUTATION, path: '/step2', component: RequestComputation, icon: NumberSquareTwoIcon },
  { step: WizardStep.ACTIVATE_E3, path: '/step3', component: ActivateE3, icon: NumberSquareThreeIcon },
  { step: WizardStep.ENTER_INPUTS, path: '/step4', component: EnterInputs, icon: NumberSquareFourIcon },
  { step: WizardStep.ENCRYPT_SUBMIT, path: '/step5', component: EncryptSubmit, icon: NumberSquareFiveIcon },
  { step: WizardStep.RESULTS, path: '/step6', component: Results, icon: NumberSquareSixIcon },
]

/**
 * WizardRoutes component that manages the multi-step wizard flow for Enclave E3.
 * Handles routing between wizard steps, displays step indicators, and manages
 * error states for environment configuration and SDK errors.
 *
 * Dynamically sets up routes for each wizard step, only rendering the component
 * for the current step and redirecting to the currentStep's route otherwise.
 * This enforces linear navigation through the wizard.
 */
const WizardRoutes: React.FC = () => {
  const { currentStep, sdk } = useWizard()

  // Memoize the current step path to avoid unnecessary recalculations.
  const currentStepPath = useMemo(() => `/step${currentStep}`, [currentStep])

  // Early returns for error states.
  if (MISSING_ENV_VARS.length > 0) {
    return <EnvironmentError missingVars={MISSING_ENV_VARS} />
  }

  if (sdk.error) {
    return <SDKErrorDisplay error={sdk.error} />
  }

  return (
    <div className='min-h-screen bg-gradient-to-br from-slate-50 to-slate-100 text-slate-900'>
      <div className='container mx-auto px-4 py-8'>
        <div className='mb-8 text-center'>
          <h1 className='mb-2 text-4xl font-bold text-slate-800'>Enclave E3</h1>
          <p className='text-lg text-slate-600'>Confidential computation with Enclave Encrypted Execution Environments.</p>
        </div>

        <StepIndicator currentStep={currentStep} steps={STEPS} />

        <div className='mx-auto max-w-2xl'>
          <Routes>
            <Route path='/' element={<Navigate to={currentStepPath} replace />} />
            {STEPS.map(({ step, path, component: Component }) => (
              <Route key={path} path={path} element={currentStep === step ? <Component /> : <Navigate to={currentStepPath} replace />} />
            ))}
          </Routes>
        </div>
      </div>
    </div>
  )
}

export default WizardRoutes
