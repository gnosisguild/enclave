// Sepolia public client + contract addresses.
// Addresses sourced from packages/enclave-contracts/deployed_contracts.json
// and examples/CRISP/packages/crisp-contracts/deployed_contracts.json.
//
// ABIs are imported from the canonical typechain factories in
// @enclave-e3/contracts so they cannot drift from the deployed contracts.
// E3Stage / FailureReason enums come from @enclave-e3/sdk.

import { createPublicClient, http, type Address } from 'viem'
import { sepolia } from 'viem/chains'
import { CiphernodeRegistryOwnable__factory, Enclave__factory } from '@enclave-e3/contracts/types'

// Import from the /contracts subpath (not the main barrel) so we don't pull
// in the SDK's crypto stack (@aztec/bb.js, Noir ACVM wasm — ~3.5 MB).
export { E3Stage, FailureReason } from '@enclave-e3/sdk/contracts'

const RPC_URL = (import.meta as any).env?.VITE_SEPOLIA_RPC || 'https://ethereum-sepolia.publicnode.com'

export const publicClient = createPublicClient({
  chain: sepolia,
  transport: http(RPC_URL, { batch: true }),
})

export const CONTRACTS = {
  Enclave: '0xB47B267876B60a06138Bc9dfCee7aa3E26907CCB' as Address,
  CiphernodeRegistry: '0x497Feea9abB72229aab1584c22b5416ff128926B' as Address,
  BondingRegistry: '0x788046999530304DDe121e19eD456180Aca6B7c1' as Address,
  E3RefundManager: '0x91eebD89bb00CBE9E5Eabf2a01a22704f88AD098' as Address,
  CRISPProgram: '0xba3B07aBFd0B8cad68aa1E946CC7AF5C1B1c8B5D' as Address,
  FeeToken: '0x9B1820D75bb09433D17C674A289fc6dD53e9c389' as Address, // MockUSDC
}

// First block where the Enclave proxy is deployed — lower bound for getLogs.
export const DEPLOY_BLOCK = 10697349n

export const enclaveAbi = Enclave__factory.abi
export const ciphernodeRegistryAbi = CiphernodeRegistryOwnable__factory.abi
