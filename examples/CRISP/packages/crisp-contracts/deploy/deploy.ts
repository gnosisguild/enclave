// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { deployEnclave, readDeploymentArgs, updateE3Config } from '@enclave-e3/contracts/scripts'
import { deployCRISPContracts } from './crisp'
import path from 'path'

import hre from 'hardhat'
import { fileURLToPath } from 'url'

// Map contract names to config keys
const contractMapping: Record<string, string> = {
  CRISPProgram: 'e3_program',
  Enclave: 'enclave',
  CiphernodeRegistryOwnable: 'ciphernode_registry',
  BondingRegistry: 'bonding_registry',
  SlashingManager: 'slashing_manager',
  MockUSDC: 'fee_token',
  DkgFoldAttestationVerifier: 'dkg_fold_attestation_verifier',
}

// Get __dirname equivalent in ES modules
const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

/**
 * Deploys the Enclave and CRISP contracts
 */
export const deploy = async () => {
  const chain = hre.globalOptions.network

  const shouldDeployEnclave = Boolean(process.env.DEPLOY_ENCLAVE)
  const shouldPrintEnv = Boolean(process.env.PRINT_ENV_VARS)

  // Mock BFV verifiers only: CRISP E2E uses E3_PROOF_AGGREGATION_ENABLED=false and does not
  // ship compiled `*.vk_recursive_hash` artifacts required by BfvPkVerifier / BfvDecryptionVerifier.
  if (shouldDeployEnclave) {
    await deployEnclave(true, false)
  }
  await deployCRISPContracts()

  // this expects you to run it from CRISP's root
  updateE3Config(chain, path.join(__dirname, '..', '..', '..', 'enclave.config.yaml'), contractMapping)

  if (shouldPrintEnv) {
    const enclaveAddress = readDeploymentArgs('Enclave', chain)?.address
    const feeTokenAddress = readDeploymentArgs('MockUSDC', chain)?.address
    const programAddress = readDeploymentArgs('CRISPProgram', chain)?.address
    const ciphernodeRegistryAddress = readDeploymentArgs('CiphernodeRegistryOwnable', chain)?.address
    const votingTokenAddress = readDeploymentArgs('MockVotingToken', chain)?.address

    if (!enclaveAddress || !feeTokenAddress || !programAddress || !ciphernodeRegistryAddress || !votingTokenAddress) {
      console.error('Error: Missing deployment addresses. Ensure all contracts are deployed.')
      return
    }

    console.log('\nAdd these to examples/CRISP/server/.env (and client/.env for VITE_CRISP_TOKEN):')
    console.log(
      [
        `ENCLAVE_ADDRESS=${enclaveAddress}`,
        `FEE_TOKEN_ADDRESS=${feeTokenAddress}`,
        `E3_PROGRAM_ADDRESS=${programAddress}`,
        `CIPHERNODE_REGISTRY_ADDRESS=${ciphernodeRegistryAddress}`,
        `CRISP_VOTING_TOKEN=${votingTokenAddress}`,
        `VITE_CRISP_TOKEN=${votingTokenAddress}`,
      ].join('\n'),
    )
  }
}

deploy().catch((err) => {
  console.error(err)
  process.exit(1)
})
