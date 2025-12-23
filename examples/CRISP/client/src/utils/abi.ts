import { parseAbi } from 'viem'

export const iERC20Abi = parseAbi([
  'function balanceOf(address owner) view returns (uint256)',
  'function mint(address to, uint256 amount) external',
])
