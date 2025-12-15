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
  MockUSDC: 'fee_token',
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

  if (shouldDeployEnclave) {
    await deployEnclave(true)
  }
  await deployCRISPContracts()

  // this expects you to run it from CRISP's root
  updateE3Config(chain, path.join(__dirname, '..', '..', '..', 'enclave.config.yaml'), contractMapping)

  if (shouldPrintEnv) {
    const enclaveAddress = readDeploymentArgs('Enclave', chain)?.address
    const tokenAddress = readDeploymentArgs('MockUSDC', chain)?.address
    const programAddress = readDeploymentArgs('CRISPProgram', chain)?.address
    const ciphernodeRegistryAddress = readDeploymentArgs('CiphernodeRegistryOwnable', chain)?.address

    console.log('\nAdd these to your server .env')
    console.log(
      `ENCLAVE_ADDRESS=${enclaveAddress}\nFEE_TOKEN_ADDRESS=${tokenAddress}\nE3_PROGRAM_ADDRESS=${programAddress}\nCIPHERNODE_REGISTRY_ADDRESS=${ciphernodeRegistryAddress}`,
    )
  }
}

deploy().catch((err) => {
  console.log(err)
})
