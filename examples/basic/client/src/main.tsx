import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { WagmiProvider, createConfig } from 'wagmi'
import { sepolia, anvil } from 'wagmi/chains'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ConnectKitProvider, getDefaultConfig } from 'connectkit'
import App from './App.tsx'

// Web3 Configuration
const walletConnectProjectId = import.meta.env.VITE_WALLETCONNECT_PROJECT_ID || ''
if (!walletConnectProjectId) {
  console.warn('VITE_WALLETCONNECT_PROJECT_ID is not set in .env file. WalletConnect will not function properly.')
}

const chains = import.meta.env.DEV ? ([sepolia, anvil] as const) : ([sepolia] as const)

const config = createConfig(
  getDefaultConfig({
    appName: 'Enclave E3',
    enableFamily: false,
    chains,
    walletConnectProjectId: walletConnectProjectId,
  }),
)

const queryClient = new QueryClient()

const connectKitOptions = import.meta.env.DEV
  ? { initialChainId: 0 }
  : { initialChainId: sepolia.id }

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <WagmiProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <ConnectKitProvider options={connectKitOptions} mode='light'>
          <App />
        </ConnectKitProvider>
      </QueryClientProvider>
    </WagmiProvider>
  </StrictMode>,
)
