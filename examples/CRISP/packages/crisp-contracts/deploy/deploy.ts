// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { deployEnclave, updateE3Config } from '@enclave-e3/contracts/scripts'
import { deployCRISPContracts } from './crisp'
import { syncCrispEnvFromDeployments } from './syncCrispEnv'
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
}

// Get __dirname equivalent in ES modules
const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

/**
 * Deploys the Enclave and CRISP contracts
 */
export const deploy = async () => {
  const chain = hre.globalOptions.network ?? 'localhost'

  const shouldDeployEnclave = Boolean(process.env.DEPLOY_ENCLAVE)
  const withZkVerification = process.env.ENABLE_ZK_VERIFICATION === 'true'

  if (shouldDeployEnclave) {
    await deployEnclave(true, withZkVerification)
  }
  await deployCRISPContracts()

  const enclaveConfigPath = path.join(__dirname, '..', '..', '..', 'enclave.config.yaml')
  updateE3Config(chain, enclaveConfigPath, contractMapping)

  syncCrispEnvFromDeployments(chain)
}

deploy().catch((err) => {
  console.error(err)
  process.exit(1)
})
