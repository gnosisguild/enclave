import { network } from 'hardhat'
import { zeroHash } from 'viem'
import { CRISPProgram, HonkVerifier, MockEnclave, PoseidonT3 } from '../types'

// Non-zero address used in the tests.
export const nonZeroAddress = '0xc6e7DF5E7b4f2A278906862b61205850344D4e7d'

export const { ethers } = await network.connect()
export const abiCoder = ethers.AbiCoder.defaultAbiCoder()

/**
 * Deploy a contract and return the address.
 * @param contractName - The name of the contract to deploy.
 * @returns The address of the deployed contract.
 */
export async function deployContract(contractName: string) {
  const contract = await ethers.deployContract(contractName)
  await contract.waitForDeployment()

  return contract
}

/**
 * Deploy PoseidonT3 and return the address.
 * @returns The address of the deployed PoseidonT3 contract.
 */
export async function deployPoseidonT3() {
  const contract = await deployContract('PoseidonT3')

  return contract as unknown as PoseidonT3
}

/**
 * Deploy MockEnclave and return the address.
 * @returns The address of the deployed MockEnclave contract.
 */
export async function deployMockEnclave() {
  const contract = await deployContract('MockEnclave')

  return contract as unknown as MockEnclave
}

/**
 * Deploy HonkVerifier and return the address.
 * @returns The address of the deployed HonkVerifier contract.
 */
export async function deployHonkVerifier() {
  const zkTranscriptLib = await deployContract('ZKTranscriptLib')

  const HonkVerifierFactory = await ethers.getContractFactory('HonkVerifier', {
    libraries: {
      'project/contracts/CRISPVerifier.sol:ZKTranscriptLib': await zkTranscriptLib.getAddress(),
    },
  })

  const honkVerifier = await HonkVerifierFactory.deploy()

  await honkVerifier.waitForDeployment()

  return honkVerifier as unknown as HonkVerifier
}

export async function deployCRISPProgram(
  contracts: { mockEnclave?: MockEnclave; honkVerifier?: HonkVerifier; poseidonT3?: PoseidonT3 } = {},
) {
  const poseidonT3 = contracts.poseidonT3 || (await deployPoseidonT3())
  const honkVerifier = contracts.honkVerifier || (await deployHonkVerifier())
  const mockEnclave = contracts.mockEnclave || (await deployMockEnclave())

  const programFactory = await ethers.getContractFactory('CRISPProgram', {
    libraries: {
      'npm/poseidon-solidity@0.0.5/PoseidonT3.sol:PoseidonT3': await poseidonT3.getAddress(),
    },
  })

  const program = await programFactory.deploy(await mockEnclave.getAddress(), nonZeroAddress, await honkVerifier.getAddress(), zeroHash)

  await program.waitForDeployment()

  return program as CRISPProgram
}
