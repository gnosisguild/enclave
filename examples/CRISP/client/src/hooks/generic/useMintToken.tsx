// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { iERC20Abi } from '@/utils/abi'
import { useCallback, useState } from 'react'
import { usePublicClient, useWalletClient } from 'wagmi'
import { getChain } from '@/utils/methods'
import { useNotificationAlertContext } from '@/context/NotificationAlert'
import { ROUND_TOKEN } from '@/utils/constants'

/**
 * Hooks to interact with the token contract used to gatekeep access to voting rounds
 */
const useToken = () => {
  const { showToast } = useNotificationAlertContext()
  const [isMinting, setIsMinting] = useState<boolean>(false)

  const { data: walletClient } = useWalletClient()
  const publicClient = usePublicClient()

  const mintTokens = useCallback(async () => {
    if (!walletClient || !publicClient) {
      showToast({
        type: 'danger',
        message: 'Wallet not connected',
      })
      return
    }

    const balance = await publicClient.readContract({
      abi: iERC20Abi,
      address: ROUND_TOKEN,
      functionName: 'balanceOf',
      args: [walletClient.account.address],
    })

    if (balance && balance > BigInt(0)) {
      showToast({
        type: 'info',
        message: 'You already have tokens, no need to mint more',
      })
      return
    }

    setIsMinting(true)

    try {
      await walletClient.writeContract({
        abi: iERC20Abi,
        address: ROUND_TOKEN,
        functionName: 'mint',
        args: [walletClient.account.address, BigInt(1 * 1e9)],
        chain: getChain(),
      })
      showToast({
        type: 'success',
        message: 'Tokens minted successfully',
      })
    } catch (error) {
      console.log(error)
      showToast({
        type: 'danger',
        message: 'Error minting tokens',
      })
    } finally {
      setIsMinting(false)
    }
  }, [walletClient, showToast, publicClient])

  return {
    isMinting,
    mintTokens,
  }
}

export default useToken
