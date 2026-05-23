// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { WagmiProvider, createConfig, http } from 'wagmi'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ConnectKitProvider, getDefaultConfig } from 'connectkit'
import React from 'react'
import { getChain } from '@/utils/methods'

const walletConnectProjectId = import.meta.env.VITE_WALLETCONNECT_PROJECT_ID || ''
if (!walletConnectProjectId)
  console.warn('VITE_WALLETCONNECT_PROJECT_ID is not set in .env file. WalletConnect will not function properly.')

const chain = getChain()
const rpcUrl = import.meta.env.VITE_RPC_URL || 'http://127.0.0.1:8545'

const config = createConfig(
  getDefaultConfig({
    appName: 'CRISP',
    chains: [chain],
    transports: {
      [chain.id]: http(rpcUrl),
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
