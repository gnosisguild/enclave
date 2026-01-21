// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { WalletClient } from 'viem'
import { getCheckedEnvVars } from './utils'
import { MyProgram__factory as MyProgram } from '../types/factories/contracts'

/**
 * Publish an input to the program
 * @param walletClient - The wallet client to use for the transaction
 * @param e3Id - The E3 ID
 * @param input - The input data
 * @param sender - The sender address
 */
export const publishInput = async (
  walletClient: WalletClient,
  e3Id: bigint,
  input: `0x${string}`,
  sender: `0x${string}`,
): Promise<void> => {
  const { E3_PROGRAM_ADDRESS: programAddress } = getCheckedEnvVars()

  await walletClient.writeContract({
    address: programAddress as `0x${string}`,
    abi: MyProgram.abi,
    functionName: 'publishInput',
    args: [e3Id, sender, input],
    chain: walletClient.chain,
    account: sender,
  })
}
