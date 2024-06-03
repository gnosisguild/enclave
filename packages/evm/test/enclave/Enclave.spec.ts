import { loadFixture, time } from "@nomicfoundation/hardhat-network-helpers";
import { expect } from "chai";
import { ethers } from "hardhat";

import { deployMockComputationModuleFixture } from "../mocks/MockComputationModule.fixture";
import { deployMockCypherNodeRegistryFixture } from "../mocks/MockCypherNodeRegistry.fixture";
import { deployMockExecutionModuleFixture } from "../mocks/MockExecutionModule.fixture";
import { deployMockInputValidatorFixture } from "../mocks/MockInputValidator.fixture";
import { deployMockOutputVerifierFixture } from "../mocks/MockOutputVerifier.fixture";
import type { Signers } from "../types";
import { deployEnclaveFixture } from "./Enclave.fixture";

const abiCoder = ethers.AbiCoder.defaultAbiCoder();
const AddressTwo = "0x0000000000000000000000000000000000000002";

describe("Enclave", function () {
  before(async function () {
    this.signers = {} as Signers;

    const signers = await ethers.getSigners();
    this.signers.admin = signers[0];
  });

  beforeEach(async function () {
    const enclave = await loadFixture(deployEnclaveFixture);
    this.enclave = enclave;

    const { mockComputationModule, mockComputationModule_address } = await loadFixture(
      deployMockComputationModuleFixture,
    );
    this.mockComputationModule = mockComputationModule;
    this.mockComputationModule_address = mockComputationModule_address;

    const { mockOutputVerifier, mockOutputVerifier_address } = await loadFixture(deployMockOutputVerifierFixture);
    this.mockOutputVerifier = mockOutputVerifier;
    this.mockOutputVerifier_address = mockOutputVerifier_address;

    const { mockCypherNodeRegistry, mockCypherNodeRegistry_address } = await loadFixture(
      deployMockCypherNodeRegistryFixture,
    );
    this.mockCypherNodeRegistry = mockCypherNodeRegistry;
    this.mockCypherNodeRegistry_address = mockCypherNodeRegistry_address;

    const { mockExecutionModule, mockExecutionModule_address } = await loadFixture(deployMockExecutionModuleFixture);
    this.mockExecutionModule = mockExecutionModule;
    this.mockExecutionModule_address = mockExecutionModule_address;

    const { mockInputValidator, mockInputValidator_address } = await loadFixture(deployMockInputValidatorFixture);
    this.mockInputValidator = mockInputValidator;
    this.mockInputValidator_address = mockInputValidator_address;

    await this.enclave.setCypherNodeRegistry(this.mockCypherNodeRegistry_address);
    await this.enclave.enableComputationModule(this.mockComputationModule_address);
    await this.enclave.enableExecutionModule(this.mockExecutionModule_address);

    this.requestParams = {
      poolId: 1n,
      threshold: [2n, 2n],
      duration: time.duration.days(30),
      computationModule: this.mockComputationModule_address,
      cMParams: abiCoder.encode(["address"], [this.mockInputValidator_address]),
      executionModule: this.mockExecutionModule_address,
      eMParams: abiCoder.encode(["address"], [this.mockOutputVerifier_address]),
    };
  });

  describe("constructor / initialize()", function () {
    it("correctly sets owner", async function () {
      const owner1 = await this.enclave.owner();
      const [, , , otherOwner, otherRegistry] = await ethers.getSigners();

      const enclave = await deployEnclaveFixture({ owner: otherOwner, registry: otherRegistry });

      // expect the owner to be the same as the one set in the fixture
      expect(owner1).to.not.equal(otherOwner);
      // expect the owner to be the same as the one set in the new deployment
      expect(await enclave.owner()).to.equal(otherOwner);
      expect(await enclave.cypherNodeRegistry()).to.equal(otherRegistry);
    });

    it("correctly sets cypherNodeRegistry address", async function () {
      const cypherNodeRegistry = await this.enclave.cypherNodeRegistry();

      expect(cypherNodeRegistry).to.equal(this.mockCypherNodeRegistry_address);
    });

    it("correctly sets max duration", async function () {
      const enclave = await deployEnclaveFixture({ maxDuration: 9876 });
      expect(await enclave.maxDuration()).to.equal(9876);
    });
  });

  describe("setMaxDuration()", function () {
    it("reverts if not called by owner", async function () {
      const [, , , notTheOwner] = await ethers.getSigners();
      await expect(this.enclave.connect(notTheOwner).setMaxDuration(1))
        .to.be.revertedWithCustomError(this.enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("set max duration correctly", async function () {
      await this.enclave.setMaxDuration(1);

      const maxDuration = await this.enclave.maxDuration();
      expect(maxDuration).to.equal(1);
    });
    it("returns true if max duration is set successfully", async function () {
      const result = await this.enclave.setMaxDuration.staticCall(1);

      expect(result).to.be.true;
    });
    it("emits MaxDurationSet event", async function () {
      await expect(this.enclave.setMaxDuration(1)).to.emit(this.enclave, "MaxDurationSet").withArgs(1);
    });
  });

  describe("setCypherNodeRegistry()", function () {
    it("reverts if not called by owner", async function () {
      const [, , , notTheOwner] = await ethers.getSigners();

      await expect(this.enclave.connect(notTheOwner).setCypherNodeRegistry(AddressTwo))
        .to.be.revertedWithCustomError(this.enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("reverts if given address(0)", async function () {
      await expect(this.enclave.setCypherNodeRegistry(ethers.ZeroAddress))
        .to.be.revertedWithCustomError(this.enclave, "InvalidCypherNodeRegistry")
        .withArgs(ethers.ZeroAddress);
    });
    it("reverts if given address is the same as the current cypherNodeRegistry", async function () {
      await expect(this.enclave.setCypherNodeRegistry(this.mockCypherNodeRegistry_address))
        .to.be.revertedWithCustomError(this.enclave, "InvalidCypherNodeRegistry")
        .withArgs(this.mockCypherNodeRegistry_address);
    });
    it("sets cypherNodeRegistry correctly", async function () {
      expect(await this.enclave.cypherNodeRegistry()).to.not.equal(AddressTwo);

      await this.enclave.setCypherNodeRegistry(AddressTwo);

      expect(await this.enclave.cypherNodeRegistry()).to.equal(AddressTwo);
    });
    it("returns true if cypherNodeRegistry is set successfully", async function () {
      const result = await this.enclave.setCypherNodeRegistry.staticCall(AddressTwo);

      expect(result).to.be.true;
    });
    it("emits CypherNodeRegistrySet event", async function () {
      await expect(this.enclave.setCypherNodeRegistry(AddressTwo))
        .to.emit(this.enclave, "CypherNodeRegistrySet")
        .withArgs(AddressTwo);
    });
  });

  describe("getE3()", function () {
    it("reverts if E3 does not exist", async function () {
      await expect(this.enclave.getE3(1)).to.be.revertedWithCustomError(this.enclave, "E3DoesNotExist").withArgs(1);
    });
    it("returns correct E3 details", async function () {
      await this.enclave.request(
        this.requestParams.poolId,
        this.requestParams.threshold,
        this.requestParams.duration,
        this.requestParams.computationModule,
        this.requestParams.cMParams,
        this.requestParams.executionModule,
        this.requestParams.eMParams,
        { value: 10 },
      );
      const e3 = await this.enclave.getE3(0);

      expect(e3.threshold).to.deep.equal(this.requestParams.threshold);
      expect(e3.expiration).to.equal(0n);
      expect(e3.computationModule).to.equal(this.requestParams.computationModule);
      expect(e3.inputValidator).to.equal(abiCoder.decode(["address"], this.requestParams.cMParams)[0]);
      expect(e3.executionModule).to.equal(this.requestParams.executionModule);
      expect(e3.outputVerifier).to.equal(abiCoder.decode(["address"], this.requestParams.eMParams)[0]);
      expect(e3.committeePublicKey).to.equal("0x");
      expect(e3.ciphertextOutput).to.equal("0x");
      expect(e3.plaintextOutput).to.equal("0x");
    });
  });

  describe("enableComputationModule()", function () {
    it("reverts if not called by owner", async function () {
      const [, , , notTheOwner] = await ethers.getSigners();
      await expect(this.enclave.connect(notTheOwner).enableComputationModule(this.mockComputationModule_address))
        .to.be.revertedWithCustomError(this.enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("reverts if computation module is already enabled", async function () {
      await expect(this.enclave.enableComputationModule(this.mockComputationModule_address))
        .to.be.revertedWithCustomError(this.enclave, "ModuleAlreadyEnabled")
        .withArgs(this.mockComputationModule_address);
    });
    it("enables computation module correctly", async function () {
      const enabled = await this.enclave.computationModules(this.mockComputationModule_address);
      expect(enabled).to.be.true;
    });
    it("returns true if computation module is enabled successfully", async function () {
      const result = await this.enclave.enableComputationModule.staticCall(AddressTwo);
      expect(result).to.be.true;
    });
    it("emits ComputationModuleEnabled event", async function () {
      await expect(this.enclave.enableComputationModule(AddressTwo))
        .to.emit(this.enclave, "ComputationModuleEnabled")
        .withArgs(AddressTwo);
    });
  });

  describe("disableComputationModule()", function () {
    it("reverts if not called by owner", async function () {
      const [, , , notTheOwner] = await ethers.getSigners();
      await expect(this.enclave.connect(notTheOwner).disableComputationModule(this.mockComputationModule_address))
        .to.be.revertedWithCustomError(this.enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("reverts if computation module is not enabled", async function () {
      await expect(this.enclave.disableComputationModule(AddressTwo))
        .to.be.revertedWithCustomError(this.enclave, "ModuleNotEnabled")
        .withArgs(AddressTwo);
    });
    it("disables computation module correctly", async function () {
      await this.enclave.disableComputationModule(this.mockComputationModule_address);

      const enabled = await this.enclave.computationModules(this.mockComputationModule_address);
      expect(enabled).to.be.false;
    });
    it("returns true if computation module is disabled successfully", async function () {
      const result = await this.enclave.disableComputationModule.staticCall(this.mockComputationModule_address);

      expect(result).to.be.true;
    });
    it("emits ComputationModuleDisabled event", async function () {
      await expect(this.enclave.disableComputationModule(this.mockComputationModule_address))
        .to.emit(this.enclave, "ComputationModuleDisabled")
        .withArgs(this.mockComputationModule_address);
    });
  });

  describe("enableExecutionModule()", function () {
    it("reverts if not called by owner", async function () {
      const [, , , notTheOwner] = await ethers.getSigners();
      await expect(this.enclave.connect(notTheOwner).enableExecutionModule(this.mockExecutionModule_address))
        .to.be.revertedWithCustomError(this.enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner.address);
    });
    it("reverts if execution module is already enabled", async function () {
      await expect(this.enclave.enableExecutionModule(this.mockExecutionModule_address))
        .to.be.revertedWithCustomError(this.enclave, "ModuleAlreadyEnabled")
        .withArgs(this.mockExecutionModule_address);
    });
    it("enables execution module correctly", async function () {
      const enabled = await this.enclave.executionModules(this.mockExecutionModule_address);
      expect(enabled).to.be.true;
    });
    it("returns true if execution module is enabled successfully", async function () {
      const result = await this.enclave.enableExecutionModule.staticCall(AddressTwo);

      expect(result).to.be.true;
    });
    it("emits ExecutionModuleEnabled event", async function () {
      await expect(this.enclave.enableExecutionModule(AddressTwo))
        .to.emit(this.enclave, "ExecutionModuleEnabled")
        .withArgs(AddressTwo);
    });
  });

  describe("disableExecutionModule()", function () {
    it("reverts if not called by owner", async function () {
      const [, , , notTheOwner] = await ethers.getSigners();

      await expect(this.enclave.connect(notTheOwner).disableExecutionModule(this.mockExecutionModule_address))
        .to.be.revertedWithCustomError(this.enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("reverts if execution module is not enabled", async function () {
      await expect(this.enclave.disableExecutionModule(AddressTwo))
        .to.be.revertedWithCustomError(this.enclave, "ModuleNotEnabled")
        .withArgs(AddressTwo);
    });
    it("disables execution module correctly", async function () {
      await this.enclave.disableExecutionModule(this.mockExecutionModule_address);

      const enabled = await this.enclave.executionModules(this.mockExecutionModule_address);
      expect(enabled).to.be.false;
    });
    it("returns true if execution module is disabled successfully", async function () {
      const result = await this.enclave.disableExecutionModule.staticCall(this.mockExecutionModule_address);

      expect(result).to.be.true;
    });
    it("emits ExecutionModuleDisabled event", async function () {
      await expect(this.enclave.disableExecutionModule(this.mockExecutionModule_address))
        .to.emit(this.enclave, "ExecutionModuleDisabled")
        .withArgs(this.mockExecutionModule_address);
    });
  });

  describe("request()", function () {
    it("reverts if msg.value is 0", async function () {
      await expect(
        this.enclave.request(
          this.requestParams.poolId,
          this.requestParams.threshold,
          this.requestParams.duration,
          this.requestParams.computationModule,
          this.requestParams.cMParams,
          this.requestParams.executionModule,
          this.requestParams.eMParams,
        ),
      ).to.be.revertedWithCustomError(this.enclave, "PaymentRequired");
    });
    it("reverts if threshold is 0", async function () {
      await expect(
        this.enclave.request(
          this.requestParams.poolId,
          [0, 2],
          this.requestParams.duration,
          this.requestParams.computationModule,
          this.requestParams.cMParams,
          this.requestParams.executionModule,
          this.requestParams.eMParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(this.enclave, "InvalidThreshold");
    });
    it("reverts if threshold is greater than number", async function () {
      await expect(
        this.enclave.request(
          this.requestParams.poolId,
          [3, 2],
          this.requestParams.duration,
          this.requestParams.computationModule,
          this.requestParams.cMParams,
          this.requestParams.executionModule,
          this.requestParams.eMParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(this.enclave, "InvalidThreshold");
    });
    it("reverts if duration is 0", async function () {
      await expect(
        this.enclave.request(
          this.requestParams.poolId,
          this.requestParams.threshold,
          0,
          this.requestParams.computationModule,
          this.requestParams.cMParams,
          this.requestParams.executionModule,
          this.requestParams.eMParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(this.enclave, "InvalidDuration");
    });
    it("reverts if duration is greater than maxDuration", async function () {
      await expect(
        this.enclave.request(
          this.requestParams.poolId,
          this.requestParams.threshold,
          time.duration.days(31),
          this.requestParams.computationModule,
          this.requestParams.cMParams,
          this.requestParams.executionModule,
          this.requestParams.eMParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(this.enclave, "InvalidDuration");
    });
    it("reverts if computation module is not enabled", async function () {
      await expect(
        this.enclave.request(
          this.requestParams.poolId,
          this.requestParams.threshold,
          this.requestParams.duration,
          ethers.ZeroAddress,
          this.requestParams.cMParams,
          this.requestParams.executionModule,
          this.requestParams.eMParams,
          { value: 10 },
        ),
      )
        .to.be.revertedWithCustomError(this.enclave, "ComputationModuleNotAllowed")
        .withArgs(ethers.ZeroAddress);
    });
    it("reverts if execution module is not enabled", async function () {
      await expect(
        this.enclave.request(
          this.requestParams.poolId,
          this.requestParams.threshold,
          this.requestParams.duration,
          this.requestParams.computationModule,
          this.requestParams.cMParams,
          ethers.ZeroAddress,
          this.requestParams.eMParams,
          { value: 10 },
        ),
      )
        .to.be.revertedWithCustomError(this.enclave, "ModuleNotEnabled")
        .withArgs(ethers.ZeroAddress);
    });
    it("reverts if input computation module does not return input validator address", async function () {
      const zeroInput = abiCoder.encode(["address"], [ethers.ZeroAddress]);

      await expect(
        this.enclave.request(
          this.requestParams.poolId,
          this.requestParams.threshold,
          this.requestParams.duration,
          this.requestParams.computationModule,
          zeroInput,
          this.requestParams.executionModule,
          this.requestParams.eMParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(this.enclave, "InvalidComputation");
    });
    it("reverts if input execution module does not return output verifier address", async function () {
      const zeroInput = abiCoder.encode(["address"], [ethers.ZeroAddress]);

      await expect(
        this.enclave.request(
          this.requestParams.poolId,
          this.requestParams.threshold,
          this.requestParams.duration,
          this.requestParams.computationModule,
          this.requestParams.cMParams,
          this.requestParams.executionModule,
          zeroInput,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(this.enclave, "InvalidExecutionModuleSetup");
    });
    it("reverts if committee selection fails", async function () {
      await expect(
        this.enclave.request(
          0,
          this.requestParams.threshold,
          this.requestParams.duration,
          this.requestParams.computationModule,
          this.requestParams.cMParams,
          this.requestParams.executionModule,
          this.requestParams.eMParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(this.enclave, "CommitteeSelectionFailed");
    });
    it("instantiates a new E3", async function () {
      await this.enclave.request(
        this.requestParams.poolId,
        this.requestParams.threshold,
        this.requestParams.duration,
        this.requestParams.computationModule,
        this.requestParams.cMParams,
        this.requestParams.executionModule,
        this.requestParams.eMParams,
        { value: 10 },
      );
      const e3 = await this.enclave.getE3(0);

      expect(e3.threshold).to.deep.equal(this.requestParams.threshold);
      expect(e3.expiration).to.equal(0n);
      expect(e3.computationModule).to.equal(this.requestParams.computationModule);
      expect(e3.inputValidator).to.equal(abiCoder.decode(["address"], this.requestParams.cMParams)[0]);
      expect(e3.executionModule).to.equal(this.requestParams.executionModule);
      expect(e3.outputVerifier).to.equal(abiCoder.decode(["address"], this.requestParams.eMParams)[0]);
      expect(e3.committeePublicKey).to.equal("0x");
      expect(e3.ciphertextOutput).to.equal("0x");
      expect(e3.plaintextOutput).to.equal("0x");
    });
    it("emits E3Requested event", async function () {
      const tx = await this.enclave.request(
        this.requestParams.poolId,
        this.requestParams.threshold,
        this.requestParams.duration,
        this.requestParams.computationModule,
        this.requestParams.cMParams,
        this.requestParams.executionModule,
        this.requestParams.eMParams,
        { value: 10 },
      );
      const e3 = await this.enclave.getE3(0);

      await expect(tx)
        .to.emit(this.enclave, "E3Requested")
        .withArgs(0, e3, 1, this.requestParams.computationModule, this.requestParams.executionModule);
    });
  });

  describe("activate()", function () {
    it("reverts if E3 does not exist");
    it("reverts if E3 has already been activated");
    it("reverts if cypherNodeRegistry does not return a public key");
    it("sets committeePublicKey correctly");
    it("returns true if E3 is activated successfully");
    it("emits E3Activated event");
  });

  describe("publishInput()", function () {
    it("reverts if E3 does not exist");
    it("reverts if E3 has not been activated");
    it("reverts if outside of input window");
    it("reverts if input is not valid");
    it("sets ciphertextInput correctly");
    it("returns true if input is published successfully");
    it("emits InputPublished event");
  });

  describe("publishCiphertextOutput()", function () {
    it("reverts if E3 does not exist");
    it("reverts if E3 has not been activated");
    it("reverts if input deadline has not passed");
    it("reverts if output has already been published");
    it("reverts if output is not valid");
    it("sets ciphertextOutput correctly");
    it("returns true if output is published successfully");
    it("emits CiphertextOutputPublished event");
  });

  describe("publishPlaintextOutput()", function () {
    it("reverts if E3 does not exist");
    it("reverts if E3 has not been activated");
    it("reverts if ciphertextOutput has not been published");
    it("reverts if plaintextOutput has already been published");
    it("reverts if output is not valid");
    it("sets plaintextOutput correctly");
    it("returns true if output is published successfully");
    it("emits PlaintextOutputPublished event");
  });
});
