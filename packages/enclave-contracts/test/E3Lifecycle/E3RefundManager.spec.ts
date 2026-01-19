// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import type { Signer } from "ethers";
import { network } from "hardhat";

import BondingRegistryModule from "../../ignition/modules/bondingRegistry";
import CiphernodeRegistryModule from "../../ignition/modules/ciphernodeRegistry";
import E3RefundManagerModule from "../../ignition/modules/e3RefundManager";
import EnclaveModule from "../../ignition/modules/enclave";
import EnclaveTicketTokenModule from "../../ignition/modules/enclaveTicketToken";
import EnclaveTokenModule from "../../ignition/modules/enclaveToken";
import MockDecryptionVerifierModule from "../../ignition/modules/mockDecryptionVerifier";
import MockE3ProgramModule from "../../ignition/modules/mockE3Program";
import MockStableTokenModule from "../../ignition/modules/mockStableToken";
import SlashingManagerModule from "../../ignition/modules/slashingManager";
import {
  BondingRegistry__factory as BondingRegistryFactory,
  CiphernodeRegistryOwnable__factory as CiphernodeRegistryOwnableFactory,
  E3RefundManager__factory as E3RefundManagerFactory,
  Enclave__factory as EnclaveFactory,
  EnclaveToken__factory as EnclaveTokenFactory,
  MockDecryptionVerifier__factory as MockDecryptionVerifierFactory,
  MockE3Program__factory as MockE3ProgramFactory,
  MockUSDC__factory as MockUSDCFactory,
} from "../../types";

const { ethers, ignition, networkHelpers } = await network.connect();
const { loadFixture, time } = networkHelpers;

describe("E3RefundManager", function () {
  // Time constants
  const ONE_HOUR = 60 * 60;
  const ONE_DAY = 24 * ONE_HOUR;
  const THREE_DAYS = 3 * ONE_DAY;
  const SEVEN_DAYS = 7 * ONE_DAY;
  const THIRTY_DAYS = 30 * ONE_DAY;
  const SORTITION_SUBMISSION_WINDOW = 10;

  const addressOne = "0x0000000000000000000000000000000000000001";

  // Default timeout configuration
  const defaultTimeoutConfig = {
    committeeFormationWindow: ONE_DAY,
    dkgWindow: ONE_DAY,
    computeWindow: THREE_DAYS,
    decryptionWindow: ONE_DAY,
    gracePeriod: ONE_HOUR,
  };

  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  const polynomial_degree = ethers.toBigInt(2048);
  const plaintext_modulus = ethers.toBigInt(1032193);
  const moduli = [ethers.toBigInt("18014398492704769")];

  const encodedE3ProgramParams = abiCoder.encode(
    ["uint256", "uint256", "uint256[]"],
    [polynomial_degree, plaintext_modulus, moduli],
  );

  const encryptionSchemeId =
    "0x2c2a814a0495f913a3a312fc4771e37552bc14f8a2d4075a08122d356f0849c6";

  const setup = async () => {
    const [
      owner,
      requester,
      treasury,
      operator1,
      operator2,
      honestNode,
      faultyNode,
    ] = await ethers.getSigners();

    const ownerAddress = await owner.getAddress();
    const treasuryAddress = await treasury.getAddress();
    const requesterAddress = await requester.getAddress();

    // Deploy USDC mock
    const usdcContract = await ignition.deploy(MockStableTokenModule, {
      parameters: {
        MockUSDC: {
          initialSupply: 10000000,
        },
      },
    });
    const usdcToken = MockUSDCFactory.connect(
      await usdcContract.mockUSDC.getAddress(),
      owner,
    );

    // Deploy ENCL token
    const enclTokenContract = await ignition.deploy(EnclaveTokenModule, {
      parameters: {
        EnclaveToken: {
          owner: ownerAddress,
        },
      },
    });
    const enclToken = EnclaveTokenFactory.connect(
      await enclTokenContract.enclaveToken.getAddress(),
      owner,
    );

    // Deploy ticket token
    const ticketTokenContract = await ignition.deploy(
      EnclaveTicketTokenModule,
      {
        parameters: {
          EnclaveTicketToken: {
            baseToken: await usdcToken.getAddress(),
            registry: addressOne,
            owner: ownerAddress,
          },
        },
      },
    );

    // Deploy slashing manager
    const slashingManagerContract = await ignition.deploy(
      SlashingManagerModule,
      {
        parameters: {
          SlashingManager: {
            admin: ownerAddress,
            bondingRegistry: addressOne,
          },
        },
      },
    );

    // Deploy bonding registry
    const bondingRegistryContract = await ignition.deploy(
      BondingRegistryModule,
      {
        parameters: {
          BondingRegistry: {
            owner: ownerAddress,
            ticketToken:
              await ticketTokenContract.enclaveTicketToken.getAddress(),
            licenseToken: await enclToken.getAddress(),
            registry: addressOne,
            slashedFundsTreasury: treasuryAddress,
            ticketPrice: ethers.parseUnits("10", 6),
            licenseRequiredBond: ethers.parseEther("1000"),
            minTicketBalance: 5,
            exitDelay: SEVEN_DAYS,
          },
        },
      },
    );
    const bondingRegistry = BondingRegistryFactory.connect(
      await bondingRegistryContract.bondingRegistry.getAddress(),
      owner,
    );

    const enclaveContract = await ignition.deploy(EnclaveModule, {
      parameters: {
        Enclave: {
          params: encodedE3ProgramParams,
          owner: ownerAddress,
          maxDuration: THIRTY_DAYS,
          registry: addressOne,
          e3RefundManager: addressOne,
          bondingRegistry: await bondingRegistry.getAddress(),
          feeToken: await usdcToken.getAddress(),
          timeoutConfig: defaultTimeoutConfig,
        },
      },
    });
    const enclaveAddress = await enclaveContract.enclave.getAddress();
    const enclave = EnclaveFactory.connect(enclaveAddress, owner);

    const ciphernodeRegistry = await ignition.deploy(CiphernodeRegistryModule, {
      parameters: {
        CiphernodeRegistry: {
          enclaveAddress: enclaveAddress,
          owner: ownerAddress,
          submissionWindow: SORTITION_SUBMISSION_WINDOW,
        },
      },
    });
    const ciphernodeRegistryAddress =
      await ciphernodeRegistry.cipherNodeRegistry.getAddress();
    const registry = CiphernodeRegistryOwnableFactory.connect(
      ciphernodeRegistryAddress,
      owner,
    );

    // Deploy E3RefundManager
    const e3RefundManagerContract = await ignition.deploy(
      E3RefundManagerModule,
      {
        parameters: {
          E3RefundManager: {
            owner: ownerAddress,
            enclave: enclaveAddress,
            feeToken: await usdcToken.getAddress(),
            bondingRegistry: await bondingRegistry.getAddress(),
            treasury: treasuryAddress,
          },
        },
      },
    );
    const e3RefundManagerAddress =
      await e3RefundManagerContract.e3RefundManager.getAddress();
    const e3RefundManager = E3RefundManagerFactory.connect(
      e3RefundManagerAddress,
      owner,
    );

    const e3ProgramContract = await ignition.deploy(MockE3ProgramModule, {
      parameters: {
        MockE3Program: {
          encryptionSchemeId: encryptionSchemeId,
        },
      },
    });
    const e3Program = MockE3ProgramFactory.connect(
      await e3ProgramContract.mockE3Program.getAddress(),
      owner,
    );

    const decryptionVerifierContract = await ignition.deploy(
      MockDecryptionVerifierModule,
    );
    const decryptionVerifier = MockDecryptionVerifierFactory.connect(
      await decryptionVerifierContract.mockDecryptionVerifier.getAddress(),
      owner,
    );

    await enclave.setCiphernodeRegistry(ciphernodeRegistryAddress);
    await enclave.setE3RefundManager(e3RefundManagerAddress);
    await enclave.enableE3Program(await e3Program.getAddress());
    await enclave.setDecryptionVerifier(
      encryptionSchemeId,
      await decryptionVerifier.getAddress(),
    );

    await bondingRegistry.setRewardDistributor(enclaveAddress);
    await bondingRegistry.setRegistry(ciphernodeRegistryAddress);
    await bondingRegistry.setSlashingManager(
      await slashingManagerContract.slashingManager.getAddress(),
    );
    await slashingManagerContract.slashingManager.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );
    await registry.setBondingRegistry(await bondingRegistry.getAddress());

    await ticketTokenContract.enclaveTicketToken.setRegistry(
      await bondingRegistry.getAddress(),
    );

    await usdcToken.mint(requesterAddress, ethers.parseUnits("10000", 6));
    await usdcToken.mint(e3RefundManagerAddress, ethers.parseUnits("10000", 6));

    const makeRequest = async (
      signer: Signer = requester,
    ): Promise<{ e3Id: number }> => {
      const startTime = (await time.latest()) + 100;

      const requestParams = {
        threshold: [2, 2] as [number, number],
        startWindow: [startTime, startTime + ONE_DAY] as [number, number],
        duration: ONE_DAY,
        e3Program: await e3Program.getAddress(),
        e3ProgramParams: encodedE3ProgramParams,
        computeProviderParams: abiCoder.encode(
          ["address"],
          [await decryptionVerifier.getAddress()],
        ),
        customParams: abiCoder.encode(
          ["address"],
          ["0x1234567890123456789012345678901234567890"],
        ),
      };

      const fee = await enclave.getE3Quote(requestParams);
      await usdcToken.connect(signer).approve(enclaveAddress, fee);
      await enclave.connect(signer).request(requestParams);

      return { e3Id: 0 };
    };

    async function setupOperator(operator: Signer) {
      const operatorAddress = await operator.getAddress();

      await enclToken.setTransferRestriction(false);
      await enclToken.mintAllocation(
        operatorAddress,
        ethers.parseEther("10000"),
        "Test allocation",
      );
      await usdcToken.mint(operatorAddress, ethers.parseUnits("100000", 6));

      await enclToken
        .connect(operator)
        .approve(await bondingRegistry.getAddress(), ethers.parseEther("2000"));
      await bondingRegistry
        .connect(operator)
        .bondLicense(ethers.parseEther("1000"));
      await bondingRegistry.connect(operator).registerOperator();

      const ticketTokenAddress = await bondingRegistry.ticketToken();
      const ticketAmount = ethers.parseUnits("100", 6);
      await usdcToken
        .connect(operator)
        .approve(ticketTokenAddress, ticketAmount);
      await bondingRegistry.connect(operator).addTicketBalance(ticketAmount);
    }

    return {
      enclave,
      e3RefundManager,
      bondingRegistry,
      registry,
      usdcToken,
      enclToken,
      e3Program,
      decryptionVerifier,
      owner,
      requester,
      treasury,
      operator1,
      operator2,
      honestNode,
      faultyNode,
      makeRequest,
      setupOperator,
    };
  };

  describe("Refund Calculation", function () {
    it("calculates refund correctly for committee formation timeout", async function () {
      const { enclave, e3RefundManager, makeRequest } =
        await loadFixture(setup);

      const { e3Id } = await makeRequest();

      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await enclave.markE3Failed(e3Id);
      await enclave.processE3Failure(e3Id);
      const distribution = await e3RefundManager.getRefundDistribution(e3Id);

      expect(distribution.requesterAmount).to.be.gt(0);
      expect(distribution.honestNodeAmount).to.equal(0);
      expect(distribution.protocolAmount).to.be.gt(0);
    });

    it("calculates refund correctly for DKG timeout", async function () {
      const {
        enclave,
        e3RefundManager,
        registry,
        makeRequest,
        setupOperator,
        operator1,
        operator2,
      } = await loadFixture(setup);

      // Setup operators
      await setupOperator(operator1);
      await setupOperator(operator2);

      const { e3Id } = await makeRequest();

      // Complete sortition but fail DKG
      await registry.connect(operator1).submitTicket(e3Id, 1);
      await registry.connect(operator2).submitTicket(e3Id, 1);
      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
      await registry.finalizeCommittee(e3Id);

      // Wait for DKG timeout
      await time.increase(defaultTimeoutConfig.dkgWindow + 1);
      await enclave.markE3Failed(e3Id);

      // Process failure
      await enclave.processE3Failure(e3Id);

      // Verify refund distribution
      const distribution = await e3RefundManager.getRefundDistribution(e3Id);

      // DKG timeout means committee formation work was done (~10% of total)
      // Requester should get most back, but some goes to honest nodes
      expect(distribution.requesterAmount).to.be.gt(0);
      expect(distribution.honestNodeAmount).to.be.gt(0);
    });
  });

  describe("claimRequesterRefund()", function () {
    it("allows requester to claim refund after E3 failure", async function () {
      const { enclave, e3RefundManager, makeRequest, requester, usdcToken } =
        await loadFixture(setup);

      await makeRequest();

      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await enclave.markE3Failed(0);
      await enclave.processE3Failure(0);

      const balanceBefore = await usdcToken.balanceOf(
        await requester.getAddress(),
      );

      await e3RefundManager.connect(requester).claimRequesterRefund(0);

      const balanceAfter = await usdcToken.balanceOf(
        await requester.getAddress(),
      );

      const distribution = await e3RefundManager.getRefundDistribution(0);
      expect(balanceAfter - balanceBefore).to.equal(
        distribution.requesterAmount,
      );
    });

    it("reverts if E3 not failed", async function () {
      const { e3RefundManager, makeRequest, requester } =
        await loadFixture(setup);

      await makeRequest();

      await expect(
        e3RefundManager.connect(requester).claimRequesterRefund(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "E3NotFailed");
    });

    it("reverts if already claimed", async function () {
      const { enclave, e3RefundManager, makeRequest, requester } =
        await loadFixture(setup);

      await makeRequest();

      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await enclave.markE3Failed(0);
      await enclave.processE3Failure(0);

      await e3RefundManager.connect(requester).claimRequesterRefund(0);

      await expect(
        e3RefundManager.connect(requester).claimRequesterRefund(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "AlreadyClaimed");
    });
  });

  describe("initialization", function () {
    it("correctly sets enclave address", async function () {
      const { enclave, e3RefundManager } = await loadFixture(setup);

      expect(await e3RefundManager.enclave()).to.equal(
        await enclave.getAddress(),
      );
    });
  });
});
