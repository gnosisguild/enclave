import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { WagmiProvider, createConfig } from 'wagmi'
import { sepolia, anvil } from 'wagmi/chains'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ConnectKitProvider, getDefaultConfig } from 'connectkit'
import App from './App.tsx'

const wagmiConfig = createConfig(
  getDefaultConfig({
    appName: 'Enclave E3',
    enableFamily: false,
    chains: import.meta.env.DEV
      ? ([sepolia, anvil] as const)
      : ([sepolia] as const),
    walletConnectProjectId: import.meta.env.VITE_WALLETCONNECT_PROJECT_ID!,
  }),
)

const queryClient = new QueryClient()
const connectKitOptions = import.meta.env.DEV
  ? { initialChainId: 0 }
  : { initialChainId: sepolia.id }

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <WagmiProvider config={wagmiConfig}>
      <QueryClientProvider client={queryClient}>
        <ConnectKitProvider options={connectKitOptions} mode='light'>
          <App />
        </ConnectKitProvider>
      </QueryClientProvider>
    </WagmiProvider>
  </StrictMode>,
)
