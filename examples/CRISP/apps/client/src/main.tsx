import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import './globals.css'
import { HashRouter } from 'react-router-dom'
import { VoteManagementProvider } from '@/context/voteManagement/index.ts'
import { NotificationAlertProvider } from './context/NotificationAlert/NotificationAlert.context.tsx'
import '@farcaster/auth-kit/styles.css'
import { AuthKitProvider } from '@farcaster/auth-kit'

const config = {
  relay: 'https://relay.farcaster.xyz',
  domain: window.location.host,
  siweUri: window.location.href,
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.Fragment>
    <HashRouter>
      <AuthKitProvider config={config}>
        <NotificationAlertProvider>
          <VoteManagementProvider>
            <App />
          </VoteManagementProvider>
        </NotificationAlertProvider>
      </AuthKitProvider>
    </HashRouter>
  </React.Fragment>,
)
