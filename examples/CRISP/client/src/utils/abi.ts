// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { parseAbi } from 'viem'

export const iERC20Abi = parseAbi([
  'function balanceOf(address owner) view returns (uint256)',
  'function mint(address to, uint256 amount) external',
])
