import { WagmiProvider, createConfig } from 'wagmi'
import { sepolia, anvil } from 'wagmi/chains'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ConnectKitProvider, getDefaultConfig } from 'connectkit'
import React from 'react'

const walletConnectProjectId = import.meta.env.VITE_WALLETCONNECT_PROJECT_ID || ''
if (!walletConnectProjectId)
  console.warn('VITE_WALLETCONNECT_PROJECT_ID is not set in .env file. WalletConnect will not function properly.')

const chains = import.meta.env.DEV ? [sepolia, anvil] as const : [sepolia] as const

const config = createConfig(
  getDefaultConfig({
    appName: 'CRISP',
    enableFamily: false,
    chains,
    walletConnectProjectId: walletConnectProjectId,
  }),
)

const queryClient = new QueryClient()

const initialChainId = 0 // NOTE: this ensures that clicking the button doesn't force the change of network which we need for testing we can drive it from an env var if required later

export const Web3Provider = ({ children }: { children: React.ReactNode }) => {
  return (
    <WagmiProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <ConnectKitProvider options={{ initialChainId }} mode='light'>
          {children}
        </ConnectKitProvider>
      </QueryClientProvider>
    </WagmiProvider>
  )
}
