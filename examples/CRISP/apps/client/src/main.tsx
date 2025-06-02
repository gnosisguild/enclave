import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import './globals.css'
import { BrowserRouter } from 'react-router-dom'
import { VoteManagementProvider } from '@/context/voteManagement/index.ts'
import { NotificationAlertProvider } from './context/NotificationAlert/NotificationAlert.context.tsx'
import { Web3Provider } from '@/providers/Web3Provider'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.Fragment>
    <BrowserRouter>
      <Web3Provider>
        <NotificationAlertProvider>
          <VoteManagementProvider>
            <App />
          </VoteManagementProvider>
        </NotificationAlertProvider>
      </Web3Provider>
    </BrowserRouter>
  </React.Fragment>,
)
