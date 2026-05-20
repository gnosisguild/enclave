// External destinations. Only URLs we can verify (from the repo's docs/llms.txt,
// README, and the Sepolia block explorer) — no guessed paths.

export const LINKS = {
  site: 'https://theinterfold.com/',
  blog: 'https://blog.theinterfold.com/',
  docs: 'https://docs.theinterfold.com/introduction',
  architecture: 'https://docs.theinterfold.com/architecture-overview',
  useCases: 'https://docs.theinterfold.com/use-cases',
  crisp: 'https://docs.theinterfold.com/CRISP/introduction',
  repo: 'https://github.com/gnosisguild/enclave',
  explorer: 'https://sepolia.etherscan.io',
} as const

export function explorerAddress(address: string): string {
  return `${LINKS.explorer}/address/${address}`
}

export function explorerTx(txHash: string): string {
  return `${LINKS.explorer}/tx/${txHash}`
}
