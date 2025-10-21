// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React from 'react'
import { BrowserRouter } from 'react-router-dom'
import WizardRoutes from './pages/WizardRoutes'
import { WizardProvider } from './context/WizardContext'
import './globals.css'

const App: React.FC = () => {
  return (
    <BrowserRouter>
      <WizardProvider>
        <WizardRoutes />
      </WizardProvider>
    </BrowserRouter>
  )
}

export default App
