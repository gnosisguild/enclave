
// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { readDeploymentArgs, storeDeploymentArgs } from "@enclave-e3/contracts/scripts";
import { Enclave__factory as EnclaveFactory } from "@enclave-e3/contracts/types";

import { execSync } from "child_process";
import hre from "hardhat";

const IMAGE_ID = "0x23734b77b0f76e85623a88d7a82f24c34c94834f2501964ea123b7a2027013a2"

export const deployCRISPContracts = async () => {
    const { ethers } = await hre.network.connect();
    const [owner] = await ethers.getSigners();

    const chain = hre.globalOptions.network;

    const useMockVerifier = Boolean(process.env.USE_MOCK_VERIFIER) ?? false;
    const useMockInputValidator = Boolean(process.env.USE_MOCK_INPUT_VALIDATOR) ?? false;

    console.log("useMockVerifier", useMockVerifier)

    const verifier = await deployVerifier(useMockVerifier)

    const inputValidator = await deployInputValidator(useMockInputValidator)

    const enclaveAddress = readDeploymentArgs("Enclave", chain)?.address;
    if (!enclaveAddress) {
        throw new Error("Enclave address not found, it must be deployed first");
    }
    const enclave = EnclaveFactory.connect(enclaveAddress, owner);

    const crispInputValidatorFactoryFactory = await ethers.getContractFactory("CRISPInputValidatorFactory")
    const crispInputValidatorFactory = await crispInputValidatorFactoryFactory.deploy(inputValidator);

    const crispInputValidatorFactoryAddress = await crispInputValidatorFactory.getAddress();
    storeDeploymentArgs({
        address: crispInputValidatorFactoryAddress,
        constructorArgs: {
            inputValidator
        }
    }, "CRISPInputValidatorFactory", chain);

    const honkVerifierFactory = await ethers.getContractFactory("HonkVerifier");
    const honkVerifier = await honkVerifierFactory.deploy();
    const honkVerifierAddress = await honkVerifier.getAddress();

    storeDeploymentArgs({
        address: honkVerifierAddress,
    }, "HonkVerifier", chain);

    const crispFactory = await ethers.getContractFactory("CRISPProgram");
    const crisp = await crispFactory.deploy(
        enclaveAddress,
        verifier,
        crispInputValidatorFactory.getAddress(),
        honkVerifierAddress,
        IMAGE_ID
    );

    const crispAddress = await crisp.getAddress();

    storeDeploymentArgs({
        address: crispAddress,
        constructorArgs: {
            enclave: enclaveAddress,
            verifierAddress: verifier,
            inputValidatorAddress: inputValidator,
            honkVerifierAddress,
            imageId: IMAGE_ID
        }
    }, "CRISPProgram", chain);

    // enable the program on Enclave
    const tx = await enclave.enableE3Program(crispAddress);
    await tx.wait();

    console.log(`
        Deployments:
        ----------------------------------------------------------------------
        Enclave: ${enclaveAddress}
        Risc0Verifier: ${verifier}
        InputValidator: ${inputValidator}
        CRISPInputValidatorFactory: ${crispInputValidatorFactoryAddress}
        HonkVerifier: ${honkVerifierAddress}
        CRISPProgram: ${crispAddress}
        `);
}

/**
 * Deploys the verifier contract
 * @param useMockVerifier - whether to use a mock verifier
 * @returns The address of the verifier
 */
export const deployVerifier = async (useMockVerifier: boolean): Promise<string> => {
    const { ethers } = await hre.network.connect();
    const chain = hre.globalOptions.network;

    if (!useMockVerifier) {
        const existingVerifier = readDeploymentArgs("RiscZeroGroth16Verifier", chain);
        if (existingVerifier?.address) {
            console.log("RiscZeroGroth16Verifier already deployed at:", existingVerifier.address);
            return existingVerifier.address;
        }

        const verifierFactory = await ethers.getContractFactory("RiscZeroGroth16Verifier");
        const verifier = await verifierFactory.deploy();
        await verifier.waitForDeployment();

        const address = await verifier.getAddress();
        
        storeDeploymentArgs({
            address,
        }, "RiscZeroGroth16Verifier", chain);

        return address;
    }

    // Check if mock verifier already deployed
    const existingMockVerifier = readDeploymentArgs("MockRISC0Verifier", chain);
    if (existingMockVerifier?.address) {
        console.log("MockRISC0Verifier already deployed at:", existingMockVerifier.address);
        return existingMockVerifier.address;
    }

    const mockVerifierFactory = await ethers.getContractFactory("MockRISC0Verifier");
    const mockVerifier = await mockVerifierFactory.deploy();

    const mockVerifierAddress = await mockVerifier.getAddress();
    storeDeploymentArgs({
        address: mockVerifierAddress,
    }, "MockRISC0Verifier", hre.globalOptions.network);

    return mockVerifierAddress;  
}


/**
 * Deploys the input validator contract
 * @param useMockInputValidator - whether to use a mock input validator
 * @returns The address of the input validator
 */
export const deployInputValidator = async (useMockInputValidator: boolean): Promise<string> => {
    const { ethers } = await hre.network.connect();
    const chain = hre.globalOptions.network;

    if (useMockInputValidator) {
        // Check if mock input validator already deployed
        const existingMockInputValidator = readDeploymentArgs("MockCRISPInputValidator", chain);
        if (existingMockInputValidator?.address) {
            console.log("MockInputValidator already deployed at:", existingMockInputValidator.address);
            return existingMockInputValidator.address;
        }

        const mockInputValidatorFactory = await ethers.getContractFactory("MockCRISPInputValidator");
        const mockInputValidator = await mockInputValidatorFactory.deploy();
        const address = await mockInputValidator.getAddress();

        storeDeploymentArgs({
            address,
        }, "MockCRISPInputValidator", hre.globalOptions.network);

        return address;
    }

    // Check if input validator already deployed
    const existingInputValidator = readDeploymentArgs("CRISPInputValidator", chain);
    if (existingInputValidator?.address) {
        console.log("CRISPInputValidator already deployed at:", existingInputValidator.address);
        return existingInputValidator.address;
    }

    const inputValidatorFactory = await ethers.getContractFactory("CRISPInputValidator");
    const inputValidator = await inputValidatorFactory.deploy();
    const address = await inputValidator.getAddress();

    storeDeploymentArgs({
        address,
    }, "CRISPInputValidator", hre.globalOptions.network);

    return address;
}
