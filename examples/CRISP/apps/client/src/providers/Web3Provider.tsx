import { WagmiProvider, createConfig } from 'wagmi'
import { sepolia, anvil } from 'wagmi/chains'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ConnectKitProvider, getDefaultConfig } from 'connectkit'
import React from 'react'

type ConnectkitOptions = React.ComponentProps<typeof ConnectKitProvider>['options']

const walletConnectProjectId = import.meta.env.VITE_WALLETCONNECT_PROJECT_ID || ''
if (!walletConnectProjectId)
  console.warn('VITE_WALLETCONNECT_PROJECT_ID is not set in .env file. WalletConnect will not function properly.')

const chains = import.meta.env.DEV ? ([sepolia, anvil] as const) : ([sepolia] as const)

const config = createConfig(
  getDefaultConfig({
    appName: 'CRISP',
    enableFamily: false,
    chains,
    walletConnectProjectId: walletConnectProjectId,
  }),
)

const queryClient = new QueryClient()

const options = import.meta.env.DEV
  ? ({
    initialChainId: anvil.id,
  } as ConnectkitOptions)
  : ({
    initialChainId: sepolia.id,
  } as ConnectkitOptions);

export const Web3Provider = ({ children }: { children: React.ReactNode }) => {
  return (
    <WagmiProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <ConnectKitProvider options={options} mode='light'>
          {children}
        </ConnectKitProvider>
      </QueryClientProvider>
    </WagmiProvider>
  )
}
