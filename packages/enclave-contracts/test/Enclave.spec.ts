// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { LeanIMT } from "@zk-kit/lean-imt";
import { expect } from "chai";
import type { Signer } from "ethers";
import { network } from "hardhat";
import { poseidon2 } from "poseidon-lite";

import BondingRegistryModule from "../ignition/modules/bondingRegistry";
import CiphernodeRegistryModule from "../ignition/modules/ciphernodeRegistry";
import E3RefundManagerModule from "../ignition/modules/e3RefundManager";
import EnclaveModule from "../ignition/modules/enclave";
import EnclaveTicketTokenModule from "../ignition/modules/enclaveTicketToken";
import EnclaveTokenModule from "../ignition/modules/enclaveToken";
import mockComputeProviderModule from "../ignition/modules/mockComputeProvider";
import MockDecryptionVerifierModule from "../ignition/modules/mockDecryptionVerifier";
import MockE3ProgramModule from "../ignition/modules/mockE3Program";
import MockStableTokenModule from "../ignition/modules/mockStableToken";
import SlashingManagerModule from "../ignition/modules/slashingManager";
import {
  BondingRegistry__factory as BondingRegistryFactory,
  CiphernodeRegistryOwnable__factory as CiphernodeRegistryOwnableFactory,
  Enclave__factory as EnclaveFactory,
  MockUSDC__factory as MockUSDCFactory,
} from "../types";
import type { Enclave } from "../types/contracts/Enclave";
import type { MockUSDC } from "../types/contracts/test/MockStableToken.sol/MockUSDC";

const { ethers, ignition, networkHelpers } = await network.connect();
const { loadFixture, time, mine } = networkHelpers;

describe("Enclave", function () {
  const THIRTY_DAYS_IN_SECONDS = 60 * 60 * 24 * 30;
  const SORTITION_SUBMISSION_WINDOW = 10;
  const addressOne = "0x0000000000000000000000000000000000000001";
  const AddressTwo = "0x0000000000000000000000000000000000000002";

  const timeoutConfig = {
    committeeFormationWindow: 3600, // 1 hour
    dkgWindow: 3600, // 1 hour
    computeWindow: 3600, // 1 hour
    decryptionWindow: 3600, // 1 hour
  };

  const inputWindowDuration = 300;

  const encryptionSchemeId =
    "0x2c2a814a0495f913a3a312fc4771e37552bc14f8a2d4075a08122d356f0849c6";
  const newEncryptionSchemeId =
    "0x0000000000000000000000000000000000000000000000000000000000000002";

  const abiCoder = ethers.AbiCoder.defaultAbiCoder();

  const polynomial_degree = ethers.toBigInt(512);
  const plaintext_modulus = ethers.toBigInt(10);
  const moduli = [
    ethers.toBigInt("0xffffee001"),
    ethers.toBigInt("0xffffc4001"),
  ];

  const encodedE3ProgramParams = abiCoder.encode(
    ["uint256", "uint256", "uint256[]"],
    [polynomial_degree, plaintext_modulus, moduli],
  );

  const data = "0xda7a";
  const proof = "0x1337";

  // Hash function used to compute the tree nodes.
  const hash = (a: bigint, b: bigint) => poseidon2([a, b]);

  const setupAndPublishCommittee = async (
    registry: any,
    e3Id: number,
    nodes: string[],
    publicKey: string,
    operator1: Signer,
    operator2: Signer,
  ): Promise<void> => {
    await registry.connect(operator1).submitTicket(e3Id, 1);
    await registry.connect(operator2).submitTicket(e3Id, 1);
    await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
    await registry.finalizeCommittee(e3Id);
    const publicKeyHash = ethers.id(publicKey);
    await registry.publishCommittee(e3Id, nodes, publicKey, publicKeyHash);
  };

  // Helper function to approve USDC and make request
  const makeRequest = async (
    enclave: Enclave,
    usdcToken: MockUSDC,
    requestParams: Parameters<Enclave["request"]>[0],
    signer?: Signer,
  ) => {
    const fee = await enclave.getE3Quote(requestParams);
    const tokenContract = signer ? usdcToken.connect(signer) : usdcToken;
    const enclaveContract = signer ? enclave.connect(signer) : enclave;

    await tokenContract.approve(await enclave.getAddress(), fee);
    return enclaveContract.request(requestParams);
  };

  async function setupOperatorForSortition(
    operator: Signer,
    bondingRegistry: any,
    licenseToken: any,
    usdcToken: any,
    ticketToken: any,
    registry: any,
  ): Promise<void> {
    const operatorAddress = await operator.getAddress();

    await licenseToken.mintAllocation(
      operatorAddress,
      ethers.parseEther("10000"),
      "Test allocation",
    );
    await usdcToken.mint(operatorAddress, ethers.parseUnits("100000", 6));

    await licenseToken
      .connect(operator)
      .approve(await bondingRegistry.getAddress(), ethers.parseEther("2000"));
    await bondingRegistry
      .connect(operator)
      .bondLicense(ethers.parseEther("1000"));
    await bondingRegistry.connect(operator).registerOperator();

    const ticketAmount = ethers.parseUnits("100", 6);
    await usdcToken
      .connect(operator)
      .approve(await ticketToken.getAddress(), ticketAmount);
    await bondingRegistry.connect(operator).addTicketBalance(ticketAmount);

    await registry.addCiphernode(operatorAddress);
  }

  const setup = async () => {
    const [owner, notTheOwner, operator1, operator2] =
      await ethers.getSigners();

    const ownerAddress = await owner.getAddress();

    const usdcContract = await ignition.deploy(MockStableTokenModule, {
      parameters: {
        MockUSDC: {
          initialSupply: 1000000,
        },
      },
    });

    const usdcToken = MockUSDCFactory.connect(
      await usdcContract.mockUSDC.getAddress(),
      owner,
    );

    const enclTokenContract = await ignition.deploy(EnclaveTokenModule, {
      parameters: {
        EnclaveToken: {
          owner: ownerAddress,
        },
      },
    });

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

    const slashingManagerContract = await ignition.deploy(
      SlashingManagerModule,
      {
        parameters: {
          SlashingManager: {
            admin: ownerAddress,
            bondingRegistry: addressOne,
            ciphernodeRegistry: addressOne,
            enclave: addressOne,
          },
        },
      },
    );

    const bondingRegistryContract = await ignition.deploy(
      BondingRegistryModule,
      {
        parameters: {
          BondingRegistry: {
            owner: ownerAddress,
            ticketToken:
              await ticketTokenContract.enclaveTicketToken.getAddress(),
            licenseToken: await enclTokenContract.enclaveToken.getAddress(),
            registry: addressOne,
            slashedFundsTreasury: ownerAddress,
            ticketPrice: ethers.parseUnits("10", 6),
            licenseRequiredBond: ethers.parseEther("1000"),
            minTicketBalance: 5,
            exitDelay: 7 * 24 * 60 * 60,
          },
        },
      },
    );

    const enclaveContract = await ignition.deploy(EnclaveModule, {
      parameters: {
        Enclave: {
          params: encodedE3ProgramParams,
          owner: ownerAddress,
          maxDuration: THIRTY_DAYS_IN_SECONDS,
          registry: addressOne,
          bondingRegistry:
            await bondingRegistryContract.bondingRegistry.getAddress(),
          e3RefundManager: addressOne, // placeholder, will be updated
          feeToken: await usdcToken.getAddress(),
          timeoutConfig,
        },
      },
    });

    const enclaveAddress = await enclaveContract.enclave.getAddress();

    const e3RefundManagerContract = await ignition.deploy(
      E3RefundManagerModule,
      {
        parameters: {
          E3RefundManager: {
            owner: ownerAddress,
            enclave: enclaveAddress,
            treasury: ownerAddress,
          },
        },
      },
    );

    const e3RefundManagerAddress =
      await e3RefundManagerContract.e3RefundManager.getAddress();

    const enclave = EnclaveFactory.connect(enclaveAddress, owner);
    await enclave.setE3RefundManager(e3RefundManagerAddress);

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

    const ciphernodeRegistryContract = CiphernodeRegistryOwnableFactory.connect(
      ciphernodeRegistryAddress,
      owner,
    );

    const bondingRegistry = BondingRegistryFactory.connect(
      await bondingRegistryContract.bondingRegistry.getAddress(),
      owner,
    );

    const registryAddress = await enclave.ciphernodeRegistry();

    if (registryAddress !== ciphernodeRegistryAddress) {
      await enclave.setCiphernodeRegistry(ciphernodeRegistryAddress);
    }

    await ciphernodeRegistryContract.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );

    await ticketTokenContract.enclaveTicketToken.setRegistry(
      await bondingRegistry.getAddress(),
    );
    await bondingRegistry.setRegistry(ciphernodeRegistryAddress);
    await bondingRegistry.setSlashingManager(
      await slashingManagerContract.slashingManager.getAddress(),
    );
    await slashingManagerContract.slashingManager.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );

    await bondingRegistry.setRewardDistributor(enclaveAddress);

    const tree = new LeanIMT(hash);

    const licenseToken = enclTokenContract.enclaveToken;
    const ticketToken = ticketTokenContract.enclaveTicketToken;

    await licenseToken.setTransferRestriction(false);

    await setupOperatorForSortition(
      operator1,
      bondingRegistry,
      licenseToken,
      usdcToken,
      ticketToken,
      ciphernodeRegistryContract,
    );
    tree.insert(BigInt(await operator1.getAddress()));

    await setupOperatorForSortition(
      operator2,
      bondingRegistry,
      licenseToken,
      usdcToken,
      ticketToken,
      ciphernodeRegistryContract,
    );
    tree.insert(BigInt(await operator2.getAddress()));

    await mine(1);

    const mockComputeProvider = await ignition.deploy(
      mockComputeProviderModule,
    );

    const decryptionVerifier = await ignition.deploy(
      MockDecryptionVerifierModule,
    );

    const e3Program = await ignition.deploy(MockE3ProgramModule);

    await enclave.enableE3Program(await e3Program.mockE3Program.getAddress());
    await enclave.setE3ProgramsParams([encodedE3ProgramParams]);
    await enclave.setDecryptionVerifier(
      encryptionSchemeId,
      await decryptionVerifier.mockDecryptionVerifier.getAddress(),
    );

    const request = {
      threshold: [2, 2] as [number, number],
      inputWindow: [
        (await time.latest()) + 10,
        (await time.latest()) + inputWindowDuration,
      ] as [number, number],
      e3Program: await e3Program.mockE3Program.getAddress(),
      e3ProgramParams: encodedE3ProgramParams,
      computeProviderParams: abiCoder.encode(
        ["address"],
        [await decryptionVerifier.mockDecryptionVerifier.getAddress()],
      ),
      customParams: abiCoder.encode(
        ["address"],
        ["0x1234567890123456789012345678901234567890"], // arbitrary address.
      ),
    };

    await usdcToken.mint(ownerAddress, ethers.parseUnits("1000000", 6));
    await usdcToken.mint(
      await notTheOwner.getAddress(),
      ethers.parseUnits("1000000", 6),
    );

    return {
      enclave,
      ciphernodeRegistryContract,
      bondingRegistry: bondingRegistry,
      ticketToken: ticketTokenContract.enclaveTicketToken,
      licenseToken: licenseToken,
      usdcToken,
      slashingManager: slashingManagerContract.slashingManager,
      tree,
      mocks: {
        decryptionVerifier: decryptionVerifier.mockDecryptionVerifier,
        e3Program: e3Program.mockE3Program,
        mockComputeProvider: mockComputeProvider.mockComputeProvider,
      },
      request,
      owner,
      notTheOwner,
      operator1,
      operator2,
    };
  };

  describe("constructor / initialize()", function () {
    it("correctly sets owner", async function () {
      const { enclave, owner } = await loadFixture(setup);
      expect(await enclave.owner()).to.equal(await owner.getAddress());
    });

    it("correctly sets ciphernodeRegistry address", async function () {
      const { enclave, ciphernodeRegistryContract } = await loadFixture(setup);
      expect(await enclave.ciphernodeRegistry()).to.equal(
        await ciphernodeRegistryContract.getAddress(),
      );
    });

    it("correctly sets max duration", async function () {
      const { enclave } = await loadFixture(setup);
      expect(await enclave.maxDuration()).to.equal(60 * 60 * 24 * 30);
    });
  });

  describe("setMaxDuration()", function () {
    it("reverts if not called by owner", async function () {
      const { enclave, notTheOwner } = await loadFixture(setup);

      await expect(
        enclave
          .connect(notTheOwner)
          .setMaxDuration(1, { from: await notTheOwner.getAddress() }),
      )
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("set max duration correctly", async function () {
      const { enclave } = await loadFixture(setup);
      await enclave.setMaxDuration(1);
      expect(await enclave.maxDuration()).to.equal(1);
    });
    it("emits MaxDurationSet event", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(enclave.setMaxDuration(1))
        .to.emit(enclave, "MaxDurationSet")
        .withArgs(1);
    });
  });

  describe("setCiphernodeRegistry()", function () {
    it("reverts if not called by owner", async function () {
      const { enclave, notTheOwner } = await loadFixture(setup);

      await expect(
        enclave.connect(notTheOwner).setCiphernodeRegistry(AddressTwo),
      )
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });

    it("reverts if given address(0)", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(enclave.setCiphernodeRegistry(ethers.ZeroAddress))
        .to.be.revertedWithCustomError(enclave, "InvalidCiphernodeRegistry")
        .withArgs(ethers.ZeroAddress);
    });

    it("reverts if given address is the same as the current ciphernodeRegistry", async function () {
      const { enclave, ciphernodeRegistryContract } = await loadFixture(setup);
      await expect(
        enclave.setCiphernodeRegistry(
          await ciphernodeRegistryContract.getAddress(),
        ),
      )
        .to.be.revertedWithCustomError(enclave, "InvalidCiphernodeRegistry")
        .withArgs(await ciphernodeRegistryContract.getAddress());
    });

    it("sets ciphernodeRegistry correctly", async function () {
      const { enclave } = await loadFixture(setup);

      expect(await enclave.ciphernodeRegistry()).to.not.equal(AddressTwo);
      await enclave.setCiphernodeRegistry(AddressTwo);
      expect(await enclave.ciphernodeRegistry()).to.equal(AddressTwo);
    });

    it("emits CiphernodeRegistrySet event", async function () {
      const { enclave } = await loadFixture(setup);

      await expect(enclave.setCiphernodeRegistry(AddressTwo))
        .to.emit(enclave, "CiphernodeRegistrySet")
        .withArgs(AddressTwo);
    });
  });

  describe("setE3ProgramsParams()", function () {
    const encodedE3ProgramsParams = [encodedE3ProgramParams];

    it("reverts if not called by owner", async function () {
      const { enclave, notTheOwner } = await loadFixture(setup);

      await expect(
        enclave
          .connect(notTheOwner)
          .setE3ProgramsParams(encodedE3ProgramsParams),
      )
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });

    it("sets E3 program parameters correctly", async function () {
      const { enclave } = await loadFixture(setup);

      await enclave.setE3ProgramsParams(encodedE3ProgramsParams);
      expect(await enclave.e3ProgramsParams(encodedE3ProgramsParams[0]!)).to.be
        .true;
    });

    it("emits AllowedE3ProgramsParamsSet event", async function () {
      const { enclave } = await loadFixture(setup);

      await expect(enclave.setE3ProgramsParams(encodedE3ProgramsParams))
        .to.emit(enclave, "AllowedE3ProgramsParamsSet")
        .withArgs(encodedE3ProgramsParams);
    });

    it("handles multiple parameters", async function () {
      const { enclave } = await loadFixture(setup);
      encodedE3ProgramsParams.push(
        "0x0000000000000000000000000000000000000000000000000000000000000001",
      );

      await enclave.setE3ProgramsParams(encodedE3ProgramsParams);

      for (const param of encodedE3ProgramsParams) {
        expect(await enclave.e3ProgramsParams(param)).to.be.true;
      }
    });
  });

  describe("getE3()", function () {
    it("reverts if E3 does not exist", async function () {
      const { enclave } = await loadFixture(setup);

      await expect(enclave.getE3(1))
        .to.be.revertedWithCustomError(enclave, "E3DoesNotExist")
        .withArgs(1);
    });

    it("returns correct E3 details", async function () {
      const { enclave, request, usdcToken } = await loadFixture(setup);

      await makeRequest(enclave, usdcToken, {
        threshold: request.threshold,
        inputWindow: request.inputWindow,
        e3Program: request.e3Program,
        e3ProgramParams: request.e3ProgramParams,
        computeProviderParams: request.computeProviderParams,
        customParams: request.customParams,
      });

      const e3 = await enclave.getE3(0);

      expect(e3.threshold).to.deep.equal(request.threshold);
      expect(e3.inputWindow[0]).to.equal(request.inputWindow[0]);
      expect(e3.inputWindow[1]).to.equal(request.inputWindow[1]);
      expect(e3.e3Program).to.equal(request.e3Program);
      expect(e3.e3ProgramParams).to.equal(request.e3ProgramParams);
      expect(e3.decryptionVerifier).to.equal(
        abiCoder.decode(["address"], request.computeProviderParams)[0],
      );
      expect(e3.committeePublicKey).to.equal(ethers.ZeroHash);
      expect(e3.ciphertextOutput).to.equal(ethers.ZeroHash);
      expect(e3.plaintextOutput).to.equal("0x");
    });
  });

  describe("getDecryptionVerifier()", function () {
    it("returns true if encryption scheme is enabled", async function () {
      const { enclave, mocks } = await loadFixture(setup);
      expect(await enclave.getDecryptionVerifier(encryptionSchemeId)).to.equal(
        await mocks.decryptionVerifier.getAddress(),
      );
    });

    it("returns false if encryption scheme is not enabled", async function () {
      const { enclave } = await loadFixture(setup);
      expect(
        await enclave.getDecryptionVerifier(newEncryptionSchemeId),
      ).to.equal(ethers.ZeroAddress);
    });
  });

  describe("setDecryptionVerifier()", function () {
    it("reverts if caller is not owner", async function () {
      const { enclave, mocks, notTheOwner } = await loadFixture(setup);

      await expect(
        enclave
          .connect(notTheOwner)
          .setDecryptionVerifier(
            encryptionSchemeId,
            await mocks.decryptionVerifier.getAddress(),
          ),
      )
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });

    it("reverts if encryption scheme is already enabled", async function () {
      const { enclave, mocks } = await loadFixture(setup);

      await expect(
        enclave.setDecryptionVerifier(
          encryptionSchemeId,
          await mocks.decryptionVerifier.getAddress(),
        ),
      )
        .to.be.revertedWithCustomError(enclave, "InvalidEncryptionScheme")
        .withArgs(encryptionSchemeId);
    });

    it("enabled decryption verifier", async function () {
      const { enclave, mocks } = await loadFixture(setup);

      expect(
        await enclave.setDecryptionVerifier(
          newEncryptionSchemeId,
          await mocks.decryptionVerifier.getAddress(),
        ),
      );
      expect(
        await enclave.getDecryptionVerifier(newEncryptionSchemeId),
      ).to.equal(await mocks.decryptionVerifier.getAddress());
    });

    it("emits EncryptionSchemeEnabled", async function () {
      const { enclave, mocks } = await loadFixture(setup);

      await expect(
        await enclave.setDecryptionVerifier(
          newEncryptionSchemeId,
          await mocks.decryptionVerifier.getAddress(),
        ),
      )
        .to.emit(enclave, "EncryptionSchemeEnabled")
        .withArgs(newEncryptionSchemeId);
    });
  });

  describe("disableEncryptionScheme()", function () {
    it("reverts if caller is not owner", async function () {
      const { enclave, notTheOwner } = await loadFixture(setup);

      await expect(
        enclave
          .connect(notTheOwner)
          .disableEncryptionScheme(encryptionSchemeId),
      )
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("reverts if encryption scheme is not already enabled", async function () {
      const { enclave } = await loadFixture(setup);

      await expect(enclave.disableEncryptionScheme(newEncryptionSchemeId))
        .to.be.revertedWithCustomError(enclave, "InvalidEncryptionScheme")
        .withArgs(newEncryptionSchemeId);
    });
    it("disables encryption scheme", async function () {
      const { enclave } = await loadFixture(setup);

      expect(await enclave.disableEncryptionScheme(encryptionSchemeId));
      expect(await enclave.getDecryptionVerifier(encryptionSchemeId)).to.equal(
        ethers.ZeroAddress,
      );
    });
    it("emits EncryptionSchemeDisabled", async function () {
      const { enclave } = await loadFixture(setup);

      await expect(await enclave.disableEncryptionScheme(encryptionSchemeId))
        .to.emit(enclave, "EncryptionSchemeDisabled")
        .withArgs(encryptionSchemeId);
    });
  });

  describe("enableE3Program()", function () {
    it("reverts if not called by owner", async function () {
      const {
        enclave,
        mocks: { e3Program },
        notTheOwner,
      } = await loadFixture(setup);

      await expect(enclave.connect(notTheOwner).enableE3Program(e3Program))
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("reverts if E3 Program is already enabled", async function () {
      const {
        enclave,
        mocks: { e3Program },
      } = await loadFixture(setup);

      await expect(enclave.enableE3Program(e3Program))
        .to.be.revertedWithCustomError(enclave, "ModuleAlreadyEnabled")
        .withArgs(e3Program);
    });
    it("enables E3 Program correctly", async function () {
      const {
        enclave,
        mocks: { e3Program },
      } = await loadFixture(setup);
      const enabled = await enclave.e3Programs(e3Program);
      expect(enabled).to.be.true;
    });
    it("emits E3ProgramEnabled event", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(enclave.enableE3Program(AddressTwo))
        .to.emit(enclave, "E3ProgramEnabled")
        .withArgs(AddressTwo);
    });
  });

  describe("disableE3Program()", function () {
    it("reverts if not called by owner", async function () {
      const {
        enclave,
        mocks: { e3Program },
        notTheOwner,
      } = await loadFixture(setup);
      await expect(enclave.connect(notTheOwner).disableE3Program(e3Program))
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("reverts if E3 Program is not enabled", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(enclave.disableE3Program(AddressTwo))
        .to.be.revertedWithCustomError(enclave, "ModuleNotEnabled")
        .withArgs(AddressTwo);
    });
    it("disables E3 Program correctly", async function () {
      const {
        enclave,
        mocks: { e3Program },
      } = await loadFixture(setup);
      await enclave.disableE3Program(e3Program);

      const enabled = await enclave.e3Programs(e3Program);
      expect(enabled).to.be.false;
    });
    it("emits E3ProgramDisabled event", async function () {
      const {
        enclave,
        mocks: { e3Program },
      } = await loadFixture(setup);
      await expect(enclave.disableE3Program(e3Program))
        .to.emit(enclave, "E3ProgramDisabled")
        .withArgs(e3Program);
    });
  });

  describe("request()", function () {
    it("reverts if USDC allowance is insufficient", async function () {
      const { enclave, request, usdcToken } = await loadFixture(setup);
      await expect(
        enclave.request({
          threshold: request.threshold,
          inputWindow: request.inputWindow,
          e3Program: request.e3Program,
          e3ProgramParams: request.e3ProgramParams,
          computeProviderParams: request.computeProviderParams,
          customParams: request.customParams,
        }),
      ).to.be.revertedWithCustomError(usdcToken, "ERC20InsufficientAllowance");
    });
    it("reverts if threshold is 0", async function () {
      const { enclave, request, usdcToken } = await loadFixture(setup);
      const fee = await enclave.getE3Quote({
        threshold: [0, 2],
        inputWindow: request.inputWindow,
        e3Program: request.e3Program,
        e3ProgramParams: request.e3ProgramParams,
        computeProviderParams: request.computeProviderParams,
        customParams: request.customParams,
      });
      await usdcToken.approve(await enclave.getAddress(), fee);
      await expect(
        enclave.request({
          threshold: [0, 2],
          inputWindow: request.inputWindow,
          e3Program: request.e3Program,
          e3ProgramParams: request.e3ProgramParams,
          computeProviderParams: request.computeProviderParams,
          customParams: request.customParams,
        }),
      )
        .to.be.revertedWithCustomError(enclave, "InvalidThreshold")
        .withArgs([0, 2]);
    });
    it("reverts if threshold is greater than number", async function () {
      const { enclave, request, usdcToken } = await loadFixture(setup);

      await expect(
        makeRequest(enclave, usdcToken, {
          threshold: [3, 2],
          inputWindow: request.inputWindow,
          e3Program: request.e3Program,
          e3ProgramParams: request.e3ProgramParams,
          computeProviderParams: request.computeProviderParams,
          customParams: request.customParams,
        }),
      )
        .to.be.revertedWithCustomError(enclave, "InvalidThreshold")
        .withArgs([3, 2]);
    });
    it("reverts if total duration is greater than maxDuration", async function () {
      const { enclave, request, usdcToken } = await loadFixture(setup);

      await expect(
        makeRequest(enclave, usdcToken, {
          threshold: [2, 3],
          inputWindow: [
            request.inputWindow[0],
            request.inputWindow[1] + time.duration.days(31),
          ],
          e3Program: request.e3Program,
          e3ProgramParams: request.e3ProgramParams,
          computeProviderParams: request.computeProviderParams,
          customParams: request.customParams,
        }),
      ).to.be.revertedWithCustomError(enclave, "InvalidDuration");
    });
    it("reverts if E3 Program is not enabled", async function () {
      const { enclave, request, usdcToken } = await loadFixture(setup);

      await expect(
        makeRequest(enclave, usdcToken, {
          threshold: [2, 3],
          inputWindow: request.inputWindow,
          e3Program: ethers.ZeroAddress,
          e3ProgramParams: request.e3ProgramParams,
          computeProviderParams: request.computeProviderParams,
          customParams: request.customParams,
        }),
      )
        .to.be.revertedWithCustomError(enclave, "E3ProgramNotAllowed")
        .withArgs(ethers.ZeroAddress);
    });
    it("reverts if given encryption scheme is not enabled", async function () {
      const { enclave, request, usdcToken } = await loadFixture(setup);
      await enclave.disableEncryptionScheme(encryptionSchemeId);
      await expect(
        makeRequest(enclave, usdcToken, {
          threshold: request.threshold,
          inputWindow: request.inputWindow,
          e3Program: request.e3Program,
          e3ProgramParams: request.e3ProgramParams,
          computeProviderParams: request.computeProviderParams,
          customParams: request.customParams,
        }),
      )
        .to.be.revertedWithCustomError(enclave, "InvalidEncryptionScheme")
        .withArgs(encryptionSchemeId);
    });
    it("instantiates a new E3", async function () {
      const { enclave, request, usdcToken } = await loadFixture(setup);

      await makeRequest(enclave, usdcToken, {
        threshold: request.threshold,
        inputWindow: request.inputWindow,
        e3Program: request.e3Program,
        e3ProgramParams: request.e3ProgramParams,
        computeProviderParams: request.computeProviderParams,
        customParams: request.customParams,
      });

      const e3 = await enclave.getE3(0);
      const block = await ethers.provider.getBlock("latest").catch((e) => e);

      expect(e3.threshold).to.deep.equal(request.threshold);
      expect(e3.inputWindow[0]).to.equal(request.inputWindow[0]);
      expect(e3.inputWindow[1]).to.equal(request.inputWindow[1]);
      expect(e3.e3Program).to.equal(request.e3Program);
      expect(e3.requestBlock).to.equal(block.number);
      expect(e3.decryptionVerifier).to.equal(
        abiCoder.decode(["address"], request.computeProviderParams)[0],
      );
      expect(e3.committeePublicKey).to.equal(ethers.ZeroHash);
      expect(e3.ciphertextOutput).to.equal(ethers.ZeroHash);
      expect(e3.plaintextOutput).to.equal("0x");
    });
    it("emits E3Requested event", async function () {
      const { enclave, request, usdcToken } = await loadFixture(setup);
      const tx = await makeRequest(enclave, usdcToken, {
        threshold: request.threshold,
        inputWindow: request.inputWindow,
        e3Program: request.e3Program,
        e3ProgramParams: request.e3ProgramParams,
        computeProviderParams: request.computeProviderParams,
        customParams: request.customParams,
      });
      const e3 = await enclave.getE3(0);

      await expect(tx)
        .to.emit(enclave, "E3Requested")
        .withArgs(0, e3, request.e3Program);
    });
  });

  describe("publishCiphertextOutput()", function () {
    it("reverts if E3 does not exist", async function () {
      const { enclave } = await loadFixture(setup);

      await expect(enclave.publishCiphertextOutput(0, "0x", "0x"))
        .to.be.revertedWithCustomError(enclave, "E3DoesNotExist")
        .withArgs(0);
    });

    it("reverts if output has already been published", async function () {
      const {
        enclave,
        request,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
      } = await loadFixture(setup);
      const e3Id = 0;

      await makeRequest(enclave, usdcToken, {
        threshold: request.threshold,
        inputWindow: request.inputWindow,
        e3Program: request.e3Program,
        e3ProgramParams: request.e3ProgramParams,
        computeProviderParams: request.computeProviderParams,
        customParams: request.customParams,
      });

      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
        operator1,
        operator2,
      );
      await mine(2, { interval: inputWindowDuration });

      await enclave.publishCiphertextOutput(e3Id, data, proof);
      await expect(enclave.publishCiphertextOutput(e3Id, data, proof))
        .to.be.revertedWithCustomError(enclave, "InvalidStage")
        .withArgs(e3Id, 3, 4);
    });
    it("reverts if committee duties are over", async function () {
      const {
        enclave,
        request,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
      } = await loadFixture(setup);
      const e3Id = 0;

      await makeRequest(enclave, usdcToken, {
        ...request,
        inputWindow: [(await time.latest()) + 20, (await time.latest()) + 100],
      });

      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
        operator1,
        operator2,
      );
      await mine(2, {
        interval: inputWindowDuration + timeoutConfig.computeWindow,
      });
      await expect(
        enclave.publishCiphertextOutput(e3Id, data, proof),
      ).to.be.revertedWithCustomError(enclave, "CommitteeDutiesCompleted");
    });
    it("reverts if output is not valid", async function () {
      const {
        enclave,
        request,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
      } = await loadFixture(setup);
      const e3Id = 0;

      await makeRequest(enclave, usdcToken, {
        threshold: request.threshold,
        inputWindow: [(await time.latest()) + 20, (await time.latest()) + 100],
        e3Program: request.e3Program,
        e3ProgramParams: request.e3ProgramParams,
        computeProviderParams: request.computeProviderParams,
        customParams: request.customParams,
      });

      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
        operator1,
        operator2,
      );
      await mine(2, { interval: inputWindowDuration });
      await expect(
        enclave.publishCiphertextOutput(e3Id, "0x", "0x"),
      ).to.be.revertedWithCustomError(enclave, "InvalidOutput");
    });
    it("sets ciphertextOutput correctly", async function () {
      const {
        enclave,
        request,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
      } = await loadFixture(setup);
      const e3Id = 0;

      await makeRequest(enclave, usdcToken, {
        ...request,
        inputWindow: [(await time.latest()) + 20, (await time.latest()) + 100],
      });

      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
        operator1,
        operator2,
      );
      await mine(2, { interval: inputWindowDuration });
      expect(await enclave.publishCiphertextOutput(e3Id, data, proof));
      const e3 = await enclave.getE3(e3Id);
      expect(e3.ciphertextOutput).to.equal(ethers.keccak256(data));
    });
    it("returns true if output is published successfully", async function () {
      const {
        enclave,
        request,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
      } = await loadFixture(setup);
      const e3Id = 0;

      await makeRequest(enclave, usdcToken, {
        ...request,
        inputWindow: [(await time.latest()) + 20, (await time.latest()) + 100],
      });

      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
        operator1,
        operator2,
      );
      await mine(2, { interval: inputWindowDuration });
      expect(
        await enclave.publishCiphertextOutput.staticCall(e3Id, data, proof),
      ).to.equal(true);
    });
    it("emits CiphertextOutputPublished event", async function () {
      const {
        enclave,
        request,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
      } = await loadFixture(setup);
      const e3Id = 0;

      await makeRequest(enclave, usdcToken, {
        ...request,
        inputWindow: [(await time.latest()) + 20, (await time.latest()) + 100],
      });

      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
        operator1,
        operator2,
      );
      await mine(2, { interval: inputWindowDuration });
      await expect(enclave.publishCiphertextOutput(e3Id, data, proof))
        .to.emit(enclave, "CiphertextOutputPublished")
        .withArgs(e3Id, data);
    });
  });

  describe("publishPlaintextOutput()", function () {
    it("reverts if E3 does not exist", async function () {
      const { enclave } = await loadFixture(setup);
      const e3Id = 0;

      await expect(enclave.publishPlaintextOutput(e3Id, data, "0x"))
        .to.be.revertedWithCustomError(enclave, "E3DoesNotExist")
        .withArgs(e3Id);
    });

    it("reverts if ciphertextOutput has not been published", async function () {
      const {
        enclave,
        request,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
      } = await loadFixture(setup);
      const e3Id = 0;

      await makeRequest(enclave, usdcToken, {
        ...request,
        inputWindow: [(await time.latest()) + 20, (await time.latest()) + 100],
      });

      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
        operator1,
        operator2,
      );
      await expect(
        enclave.publishPlaintextOutput(e3Id, data, "0x"),
      ).to.be.revertedWithCustomError(enclave, "InvalidStage");
    });
    it("reverts if plaintextOutput has already been published", async function () {
      const {
        enclave,
        request,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
      } = await loadFixture(setup);
      const e3Id = 0;

      await makeRequest(enclave, usdcToken, {
        ...request,
        inputWindow: [(await time.latest()) + 20, (await time.latest()) + 100],
      });

      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
        operator1,
        operator2,
      );
      await mine(2, { interval: inputWindowDuration });
      await enclave.publishCiphertextOutput(e3Id, data, proof);
      await enclave.publishPlaintextOutput(e3Id, data, proof);
      await expect(
        enclave.publishPlaintextOutput(e3Id, data, proof),
      ).to.be.revertedWithCustomError(enclave, "InvalidStage");
    });
    it("reverts if output is not valid", async function () {
      const {
        enclave,
        request,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
      } = await loadFixture(setup);
      const e3Id = 0;

      await makeRequest(enclave, usdcToken, {
        ...request,
        inputWindow: [(await time.latest()) + 20, (await time.latest()) + 100],
      });

      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
        operator1,
        operator2,
      );
      await mine(2, { interval: inputWindowDuration });
      await enclave.publishCiphertextOutput(e3Id, data, proof);
      await expect(enclave.publishPlaintextOutput(e3Id, data, "0x"))
        .to.be.revertedWithCustomError(enclave, "InvalidOutput")
        .withArgs(data);
    });
    it("sets plaintextOutput correctly", async function () {
      const {
        enclave,
        request,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
      } = await loadFixture(setup);
      const e3Id = 0;

      await makeRequest(enclave, usdcToken, {
        ...request,
        inputWindow: [(await time.latest()) + 20, (await time.latest()) + 100],
      });

      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
        operator1,
        operator2,
      );
      await mine(2, { interval: inputWindowDuration });
      await enclave.publishCiphertextOutput(e3Id, data, proof);
      expect(await enclave.publishPlaintextOutput(e3Id, data, proof));

      const e3 = await enclave.getE3(e3Id);
      expect(e3.plaintextOutput).to.equal(data);
    });
    it("returns true if output is published successfully", async function () {
      const {
        enclave,
        request,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
      } = await loadFixture(setup);
      const e3Id = 0;

      await makeRequest(enclave, usdcToken, {
        ...request,
        inputWindow: [(await time.latest()) + 20, (await time.latest()) + 100],
      });

      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
        operator1,
        operator2,
      );
      await mine(2, { interval: inputWindowDuration });
      await enclave.publishCiphertextOutput(e3Id, data, proof);
      expect(
        await enclave.publishPlaintextOutput.staticCall(e3Id, data, proof),
      ).to.equal(true);
    });
    it("emits PlaintextOutputPublished event", async function () {
      const {
        enclave,
        request,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
      } = await loadFixture(setup);
      const e3Id = 0;

      await makeRequest(enclave, usdcToken, {
        ...request,
        inputWindow: [(await time.latest()) + 20, (await time.latest()) + 100],
      });

      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
        operator1,
        operator2,
      );
      await mine(2, { interval: inputWindowDuration });
      await enclave.publishCiphertextOutput(e3Id, data, proof);
      await expect(await enclave.publishPlaintextOutput(e3Id, data, proof))
        .to.emit(enclave, "PlaintextOutputPublished")
        .withArgs(e3Id, data);
    });
  });
});
