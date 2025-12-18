// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { WagmiProvider, createConfig, http } from 'wagmi'
import { sepolia, anvil } from 'wagmi/chains'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ConnectKitProvider, getDefaultConfig } from 'connectkit'
import React from 'react'
import { getChain } from '@/utils/methods'

const walletConnectProjectId = import.meta.env.VITE_WALLETCONNECT_PROJECT_ID || ''
if (!walletConnectProjectId)
  console.warn('VITE_WALLETCONNECT_PROJECT_ID is not set in .env file. WalletConnect will not function properly.')

const config = createConfig(
  getDefaultConfig({
    appName: 'CRISP',
    enableFamily: false,
    chains: [getChain()],
    transports: {
      [anvil.id]: http(anvil.rpcUrls.default.http[0]),
      [sepolia.id]: http(sepolia.rpcUrls.default.http[0]),
    },
    walletConnectProjectId: walletConnectProjectId,
  }),
)

const queryClient = new QueryClient()

// NOTE: ConnectKit doesn’t ship a drop-in chain switcher UI. This just sets the initial chain.
// For a user-facing switcher, implement a small component that calls wagmi’s `useSwitchChain`.
const options = { initialChainId: getChain().id }

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
