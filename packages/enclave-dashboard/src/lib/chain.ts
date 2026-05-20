// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
// Sepolia public client + contract addresses.
// Addresses sourced from packages/enclave-contracts/deployed_contracts.json
// and examples/CRISP/packages/crisp-contracts/deployed_contracts.json.
//
// ABIs are imported from the canonical typechain factories in
// @enclave-e3/contracts so they cannot drift from the deployed contracts.

import { createPublicClient, http, type Address } from 'viem'
import { sepolia } from 'viem/chains'
import { CiphernodeRegistryOwnable__factory, Enclave__factory } from '@enclave-e3/contracts/types'

// E3 lifecycle stages — mirrors the Solidity `IEnclave.E3Stage` enum exactly.
// Defined locally (rather than imported from @enclave-e3/sdk) so the dashboard
// has no dependency on the SDK's Rust/Noir build chain when deploying.
export enum E3Stage {
  None = 0,
  Requested = 1,
  CommitteeFinalized = 2,
  KeyPublished = 3,
  CiphertextReady = 4,
  Complete = 5,
  Failed = 6,
}

// All deployment-specific values are env-overridable (see .env.example) so the
// dashboard can point at a different deployment without code changes. Defaults
// are the current Sepolia deployment from deployed_contracts.json.
const env = ((import.meta as any).env ?? {}) as Record<string, string | undefined>
const envStr = (key: string, fallback: string): string => {
  const v = env[key]
  return v && v.trim() !== '' ? v.trim() : fallback
}

const RPC_URL = envStr('VITE_SEPOLIA_RPC', 'https://ethereum-sepolia.publicnode.com')

export const publicClient = createPublicClient({
  chain: sepolia,
  transport: http(RPC_URL, { batch: true }),
})

export const CONTRACTS = {
  Enclave: envStr('VITE_ENCLAVE_ADDRESS', '0xB47B267876B60a06138Bc9dfCee7aa3E26907CCB') as Address,
  CiphernodeRegistry: envStr('VITE_CIPHERNODE_REGISTRY_ADDRESS', '0x497Feea9abB72229aab1584c22b5416ff128926B') as Address,
  CRISPProgram: envStr('VITE_CRISP_PROGRAM_ADDRESS', '0xba3B07aBFd0B8cad68aa1E946CC7AF5C1B1c8B5D') as Address,
}

// First block to scan from — lower bound for getLogs (the Enclave deploy block).
export const DEPLOY_BLOCK = BigInt(envStr('VITE_DEPLOY_BLOCK', '10697349'))

export const enclaveAbi = Enclave__factory.abi
export const ciphernodeRegistryAbi = CiphernodeRegistryOwnable__factory.abi
