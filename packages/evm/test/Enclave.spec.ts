import {
  loadFixture,
  mine,
  time,
} from "@nomicfoundation/hardhat-network-helpers";
import { expect } from "chai";
import { ZeroHash } from "ethers";
import { ethers } from "hardhat";

import { deployEnclaveFixture } from "./fixtures/Enclave.fixture";
import { deployComputationModuleFixture } from "./fixtures/MockComputationModule.fixture";
import { deployCyphernodeRegistryFixture } from "./fixtures/MockCyphernodeRegistry.fixture";
import { deployExecutionModuleFixture } from "./fixtures/MockExecutionModule.fixture";
import { deployInputValidatorFixture } from "./fixtures/MockInputValidator.fixture";
import { deployOutputVerifierFixture } from "./fixtures/MockOutputVerifier.fixture";

const abiCoder = ethers.AbiCoder.defaultAbiCoder();
const AddressTwo = "0x0000000000000000000000000000000000000002";
const AddressSix = "0x0000000000000000000000000000000000000006";

const FilterFail = AddressTwo;
const FilterOkay = AddressSix;

describe("Enclave", function () {
  async function setup() {
    const [owner, notTheOwner] = await ethers.getSigners();

    const registry = await deployCyphernodeRegistryFixture();
    const computationModule = await deployComputationModuleFixture();
    const outputVerifier = await deployOutputVerifierFixture();
    const executionModule = await deployExecutionModuleFixture();
    const inputValidator = await deployInputValidatorFixture();

    const enclave = await deployEnclaveFixture({
      owner,
      registry: await registry.getAddress(),
    });

    await enclave.enableComputationModule(await computationModule.getAddress());
    await enclave.enableExecutionModule(await executionModule.getAddress());

    return {
      owner,
      notTheOwner,
      enclave,
      mocks: {
        computationModule,
        outputVerifier,
        executionModule,
        inputValidator,
        registry,
      },
      request: {
        filter: FilterOkay,
        threshold: [2, 2] as [number, number],
        startTime: [await time.latest(), (await time.latest()) + 100] as [
          number,
          number,
        ],
        duration: time.duration.days(30),
        computationModule: await computationModule.getAddress(),
        cMParams: abiCoder.encode(
          ["address"],
          [await inputValidator.getAddress()],
        ),
        executionModule: await executionModule.getAddress(),
        eMParams: abiCoder.encode(
          ["address"],
          [await outputVerifier.getAddress()],
        ),
      },
    };
  }

  describe("constructor / initialize()", function () {
    it("correctly sets owner", async function () {
      const [, , , someSigner] = await ethers.getSigners();
      const enclave = await deployEnclaveFixture({
        owner: someSigner,
        registry: AddressTwo,
      });
      expect(await enclave.cyphernodeRegistry()).to.equal(AddressTwo);
    });

    it("correctly sets cyphernodeRegistry address", async function () {
      const [aSigner] = await ethers.getSigners();
      const enclave = await deployEnclaveFixture({
        owner: aSigner,
        registry: AddressTwo,
      });
      expect(await enclave.cyphernodeRegistry()).to.equal(AddressTwo);
    });

    it("correctly sets max duration", async function () {
      const [aSigner] = await ethers.getSigners();
      const enclave = await deployEnclaveFixture({
        owner: aSigner,
        registry: AddressTwo,
        maxDuration: 9876,
      });
      expect(await enclave.maxDuration()).to.equal(9876);
    });
  });

  describe("setMaxDuration()", function () {
    it("reverts if not called by owner", async function () {
      const { enclave, notTheOwner } = await loadFixture(setup);
      await expect(enclave.connect(notTheOwner).setMaxDuration(1))
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("set max duration correctly", async function () {
      const { enclave } = await loadFixture(setup);
      await enclave.setMaxDuration(1);
      expect(await enclave.maxDuration()).to.equal(1);
    });
    it("returns true if max duration is set successfully", async function () {
      const { enclave } = await loadFixture(setup);
      const result = await enclave.setMaxDuration.staticCall(1);
      expect(result).to.be.true;
    });
    it("emits MaxDurationSet event", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(enclave.setMaxDuration(1))
        .to.emit(enclave, "MaxDurationSet")
        .withArgs(1);
    });
  });

  describe("setCyphernodeRegistry()", function () {
    it("reverts if not called by owner", async function () {
      const { enclave, notTheOwner } = await loadFixture(setup);

      await expect(
        enclave.connect(notTheOwner).setCyphernodeRegistry(AddressTwo),
      )
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("reverts if given address(0)", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(enclave.setCyphernodeRegistry(ethers.ZeroAddress))
        .to.be.revertedWithCustomError(enclave, "InvalidCyphernodeRegistry")
        .withArgs(ethers.ZeroAddress);
    });
    it("reverts if given address is the same as the current cyphernodeRegistry", async function () {
      const {
        enclave,
        mocks: { registry },
      } = await loadFixture(setup);
      await expect(enclave.setCyphernodeRegistry(registry))
        .to.be.revertedWithCustomError(enclave, "InvalidCyphernodeRegistry")
        .withArgs(registry);
    });
    it("sets cyphernodeRegistry correctly", async function () {
      const { enclave } = await loadFixture(setup);

      expect(await enclave.cyphernodeRegistry()).to.not.equal(AddressTwo);
      await enclave.setCyphernodeRegistry(AddressTwo);
      expect(await enclave.cyphernodeRegistry()).to.equal(AddressTwo);
    });
    it("returns true if cyphernodeRegistry is set successfully", async function () {
      const { enclave } = await loadFixture(setup);

      const result = await enclave.setCyphernodeRegistry.staticCall(AddressTwo);
      expect(result).to.be.true;
    });
    it("emits CyphernodeRegistrySet event", async function () {
      const { enclave } = await loadFixture(setup);

      await expect(enclave.setCyphernodeRegistry(AddressTwo))
        .to.emit(enclave, "CyphernodeRegistrySet")
        .withArgs(AddressTwo);
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
      const { enclave, request } = await loadFixture(setup);
      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      const e3 = await enclave.getE3(0);

      expect(e3.threshold).to.deep.equal(request.threshold);
      expect(e3.expiration).to.equal(0n);
      expect(e3.computationModule).to.equal(request.computationModule);
      expect(e3.inputValidator).to.equal(
        abiCoder.decode(["address"], request.cMParams)[0],
      );
      expect(e3.executionModule).to.equal(request.executionModule);
      expect(e3.outputVerifier).to.equal(
        abiCoder.decode(["address"], request.eMParams)[0],
      );
      expect(e3.committeePublicKey).to.equal("0x");
      expect(e3.ciphertextOutput).to.equal("0x");
      expect(e3.plaintextOutput).to.equal("0x");
    });
  });

  describe("enableComputationModule()", function () {
    it("reverts if not called by owner", async function () {
      const {
        notTheOwner,
        enclave,
        mocks: { computationModule },
      } = await loadFixture(setup);

      await expect(
        enclave.connect(notTheOwner).enableComputationModule(computationModule),
      )
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("reverts if computation module is already enabled", async function () {
      const {
        enclave,
        mocks: { computationModule },
      } = await loadFixture(setup);

      await expect(enclave.enableComputationModule(computationModule))
        .to.be.revertedWithCustomError(enclave, "ModuleAlreadyEnabled")
        .withArgs(computationModule);
    });
    it("enables computation module correctly", async function () {
      const {
        enclave,
        mocks: { computationModule },
      } = await loadFixture(setup);
      const enabled = await enclave.computationModules(computationModule);
      expect(enabled).to.be.true;
    });
    it("returns true if computation module is enabled successfully", async function () {
      const { enclave } = await loadFixture(setup);
      const result =
        await enclave.enableComputationModule.staticCall(AddressTwo);
      expect(result).to.be.true;
    });
    it("emits ComputationModuleEnabled event", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(enclave.enableComputationModule(AddressTwo))
        .to.emit(enclave, "ComputationModuleEnabled")
        .withArgs(AddressTwo);
    });
  });

  describe("disableComputationModule()", function () {
    it("reverts if not called by owner", async function () {
      const {
        notTheOwner,
        enclave,
        mocks: { computationModule },
      } = await loadFixture(setup);
      await expect(
        enclave
          .connect(notTheOwner)
          .disableComputationModule(computationModule),
      )
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("reverts if computation module is not enabled", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(enclave.disableComputationModule(AddressTwo))
        .to.be.revertedWithCustomError(enclave, "ModuleNotEnabled")
        .withArgs(AddressTwo);
    });
    it("disables computation module correctly", async function () {
      const {
        enclave,
        mocks: { computationModule },
      } = await loadFixture(setup);
      await enclave.disableComputationModule(computationModule);

      const enabled = await enclave.computationModules(computationModule);
      expect(enabled).to.be.false;
    });
    it("returns true if computation module is disabled successfully", async function () {
      const {
        enclave,
        mocks: { computationModule },
      } = await loadFixture(setup);
      const result =
        await enclave.disableComputationModule.staticCall(computationModule);

      expect(result).to.be.true;
    });
    it("emits ComputationModuleDisabled event", async function () {
      const {
        enclave,
        mocks: { computationModule },
      } = await loadFixture(setup);
      await expect(enclave.disableComputationModule(computationModule))
        .to.emit(enclave, "ComputationModuleDisabled")
        .withArgs(computationModule);
    });
  });

  describe("enableExecutionModule()", function () {
    it("reverts if not called by owner", async function () {
      const { notTheOwner, enclave } = await loadFixture(setup);
      await expect(
        enclave.connect(notTheOwner).enableExecutionModule(AddressTwo),
      )
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner.address);
    });
    it("reverts if execution module is already enabled", async function () {
      const {
        enclave,
        mocks: { executionModule },
      } = await loadFixture(setup);
      await expect(enclave.enableExecutionModule(executionModule))
        .to.be.revertedWithCustomError(enclave, "ModuleAlreadyEnabled")
        .withArgs(executionModule);
    });
    it("enables execution module correctly", async function () {
      const {
        enclave,
        mocks: { executionModule },
      } = await loadFixture(setup);
      const enabled = await enclave.executionModules(executionModule);
      expect(enabled).to.be.true;
    });
    it("returns true if execution module is enabled successfully", async function () {
      const { enclave } = await loadFixture(setup);
      const result = await enclave.enableExecutionModule.staticCall(AddressTwo);

      expect(result).to.be.true;
    });
    it("emits ExecutionModuleEnabled event", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(enclave.enableExecutionModule(AddressTwo))
        .to.emit(enclave, "ExecutionModuleEnabled")
        .withArgs(AddressTwo);
    });
  });

  describe("disableExecutionModule()", function () {
    it("reverts if not called by owner", async function () {
      const {
        notTheOwner,
        enclave,
        mocks: { executionModule },
      } = await loadFixture(setup);

      await expect(
        enclave.connect(notTheOwner).disableExecutionModule(executionModule),
      )
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("reverts if execution module is not enabled", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(enclave.disableExecutionModule(AddressTwo))
        .to.be.revertedWithCustomError(enclave, "ModuleNotEnabled")
        .withArgs(AddressTwo);
    });
    it("disables execution module correctly", async function () {
      const {
        enclave,
        mocks: { executionModule },
      } = await loadFixture(setup);

      expect(await enclave.executionModules(executionModule)).to.be.true;
      await enclave.disableExecutionModule(executionModule);
      expect(await enclave.executionModules(executionModule)).to.be.false;
    });
    it("returns true if execution module is disabled successfully", async function () {
      const {
        enclave,
        mocks: { executionModule },
      } = await loadFixture(setup);
      const result =
        await enclave.disableExecutionModule.staticCall(executionModule);

      expect(result).to.be.true;
    });
    it("emits ExecutionModuleDisabled event", async function () {
      const {
        enclave,
        mocks: { executionModule },
      } = await loadFixture(setup);

      await expect(enclave.disableExecutionModule(executionModule))
        .to.emit(enclave, "ExecutionModuleDisabled")
        .withArgs(executionModule);
    });
  });

  describe("request()", function () {
    it("reverts if msg.value is 0", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          request.duration,
          request.computationModule,
          request.cMParams,
          request.executionModule,
          request.eMParams,
        ),
      ).to.be.revertedWithCustomError(enclave, "PaymentRequired");
    });
    it("reverts if threshold is 0", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          request.filter,
          [0, 2],
          request.startTime,
          request.duration,
          request.computationModule,
          request.cMParams,
          request.executionModule,
          request.eMParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(enclave, "InvalidThreshold");
    });
    it("reverts if threshold is greater than number", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          request.filter,
          [3, 2],
          request.startTime,
          request.duration,
          request.computationModule,
          request.cMParams,
          request.executionModule,
          request.eMParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(enclave, "InvalidThreshold");
    });
    it("reverts if duration is 0", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          0,
          request.computationModule,
          request.cMParams,
          request.executionModule,
          request.eMParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(enclave, "InvalidDuration");
    });
    it("reverts if duration is greater than maxDuration", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          time.duration.days(31),
          request.computationModule,
          request.cMParams,
          request.executionModule,
          request.eMParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(enclave, "InvalidDuration");
    });
    it("reverts if computation module is not enabled", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          request.duration,
          ethers.ZeroAddress,
          request.cMParams,
          request.executionModule,
          request.eMParams,
          { value: 10 },
        ),
      )
        .to.be.revertedWithCustomError(enclave, "ComputationModuleNotAllowed")
        .withArgs(ethers.ZeroAddress);
    });
    it("reverts if execution module is not enabled", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          request.duration,
          request.computationModule,
          request.cMParams,
          ethers.ZeroAddress,
          request.eMParams,
          { value: 10 },
        ),
      )
        .to.be.revertedWithCustomError(enclave, "ModuleNotEnabled")
        .withArgs(ethers.ZeroAddress);
    });
    it("reverts if input computation module does not return input validator address", async function () {
      const { enclave, request } = await loadFixture(setup);

      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          request.duration,
          request.computationModule,
          ZeroHash,
          request.executionModule,
          request.eMParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(enclave, "InvalidComputation");
    });
    it("reverts if input execution module does not return output verifier address", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          request.duration,
          request.computationModule,
          request.cMParams,
          request.executionModule,
          ZeroHash,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(enclave, "InvalidExecutionModuleSetup");
    });
    it("reverts if committee selection fails", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          FilterFail,
          request.threshold,
          request.startTime,
          request.duration,
          request.computationModule,
          request.cMParams,
          request.executionModule,
          request.eMParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(enclave, "CommitteeSelectionFailed");
    });
    it("instantiates a new E3", async function () {
      const { enclave, request } = await loadFixture(setup);
      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      const e3 = await enclave.getE3(0);

      expect(e3.threshold).to.deep.equal(request.threshold);
      expect(e3.expiration).to.equal(0n);
      expect(e3.computationModule).to.equal(request.computationModule);
      expect(e3.inputValidator).to.equal(
        abiCoder.decode(["address"], request.cMParams)[0],
      );
      expect(e3.executionModule).to.equal(request.executionModule);
      expect(e3.outputVerifier).to.equal(
        abiCoder.decode(["address"], request.eMParams)[0],
      );
      expect(e3.committeePublicKey).to.equal("0x");
      expect(e3.ciphertextOutput).to.equal("0x");
      expect(e3.plaintextOutput).to.equal("0x");
    });
    it("emits E3Requested event", async function () {
      const { enclave, request } = await loadFixture(setup);
      const tx = await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      const e3 = await enclave.getE3(0);

      await expect(tx)
        .to.emit(enclave, "E3Requested")
        .withArgs(
          0,
          e3,
          request.filter,
          request.computationModule,
          request.executionModule,
        );
    });
  });

  describe("activate()", function () {
    it("reverts if E3 does not exist", async function () {
      const { enclave } = await loadFixture(setup);

      await expect(enclave.activate(0))
        .to.be.revertedWithCustomError(enclave, "E3DoesNotExist")
        .withArgs(0);
    });
    it("reverts if E3 has already been activated", async function () {
      const { enclave, request } = await loadFixture(setup);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      await expect(enclave.getE3(0)).to.not.be.reverted;
      await expect(enclave.activate(0)).to.not.be.reverted;
      await expect(enclave.activate(0))
        .to.be.revertedWithCustomError(enclave, "E3AlreadyActivated")
        .withArgs(0);
    });
    it("reverts if E3 is not yet ready to start", async function () {
      const { enclave, request } = await loadFixture(setup);
      const startTime = [
        (await time.latest()) + 1000,
        (await time.latest()) + 2000,
      ] as [number, number];

      await enclave.request(
        request.filter,
        request.threshold,
        startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      await expect(enclave.activate(0)).to.be.revertedWithCustomError(
        enclave,
        "E3NotReady",
      );
    });
    it("reverts if E3 start has expired", async function () {
      const { enclave, request } = await loadFixture(setup);
      const startTime = [await time.latest(), (await time.latest()) + 1] as [
        number,
        number,
      ];

      await enclave.request(
        request.filter,
        request.threshold,
        startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      mine(1, { interval: 1000 });

      await expect(enclave.activate(0)).to.be.revertedWithCustomError(
        enclave,
        "E3Expired",
      );
    });
    it("reverts if cyphernodeRegistry does not return a public key", async function () {
      const { enclave, request } = await loadFixture(setup);
      const startTime = [
        (await time.latest()) + 1000,
        (await time.latest()) + 2000,
      ] as [number, number];

      await enclave.request(
        request.filter,
        request.threshold,
        startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      await expect(enclave.activate(0)).to.be.revertedWithCustomError(
        enclave,
        "E3NotReady",
      );
    });
    it("reverts if E3 start has expired", async function () {
      const { enclave, request } = await loadFixture(setup);
      const startTime = [await time.latest(), (await time.latest()) + 1] as [
        number,
        number,
      ];

      await enclave.request(
        request.filter,
        request.threshold,
        startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      mine(1, { interval: 1000 });

      await expect(enclave.activate(0)).to.be.revertedWithCustomError(
        enclave,
        "E3Expired",
      );
    });
    it("reverts if cyphernodeRegistry does not return a public key", async function () {
      const { enclave, request } = await loadFixture(setup);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      const prevRegistry = await enclave.cyphernodeRegistry();
      const nextRegistry = await deployCyphernodeRegistryFixture(
        "MockCyphernodeRegistryEmptyKey",
      );

      await enclave.setCyphernodeRegistry(nextRegistry);
      await expect(enclave.activate(0)).to.be.revertedWithCustomError(
        enclave,
        "CommitteeSelectionFailed",
      );

      await enclave.setCyphernodeRegistry(prevRegistry);
      await expect(enclave.activate(0)).to.not.be.reverted;
    });
    it("sets committeePublicKey correctly", async () => {
      const {
        enclave,
        request,
        mocks: { registry },
      } = await loadFixture(setup);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      const e3Id = 0;
      const publicKey = await registry.committeePublicKey(e3Id);

      let e3 = await enclave.getE3(e3Id);
      expect(e3.committeePublicKey).to.not.equal(publicKey);

      await enclave.activate(e3Id);

      e3 = await enclave.getE3(e3Id);
      expect(e3.committeePublicKey).to.equal(publicKey);
    });
    it("returns true if E3 is activated successfully", async () => {
      const { enclave, request } = await loadFixture(setup);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      const e3Id = 0;

      expect(await enclave.activate.staticCall(e3Id)).to.be.equal(true);
    });
    it("emits E3Activated event", async () => {
      const { enclave, request } = await loadFixture(setup);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      const e3Id = 0;
      const e3 = await enclave.getE3(e3Id);

      await expect(enclave.activate(e3Id))
        .to.emit(enclave, "E3Activated")
        .withArgs(e3Id, e3.expiration, e3.committeePublicKey);
    });
  });

  describe("publishInput()", function () {
    it("reverts if E3 does not exist", async function () {
      const { enclave } = await loadFixture(setup);

      await expect(enclave.publishInput(0, "0x"))
        .to.be.revertedWithCustomError(enclave, "E3DoesNotExist")
        .withArgs(0);
    });

    it("reverts if E3 has not been activated", async function () {
      const { enclave, request } = await loadFixture(setup);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      const inputData = abiCoder.encode(["bytes32"], [ZeroHash]);

      await expect(enclave.getE3(0)).to.not.be.reverted;
      await expect(enclave.publishInput(0, inputData))
        .to.be.revertedWithCustomError(enclave, "E3NotActivated")
        .withArgs(0);

      await enclave.activate(0);

      await expect(enclave.publishInput(0, inputData)).to.not.be.reverted;
    });

    it("reverts if input is not valid", async function () {
      const { enclave, request } = await loadFixture(setup);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      await enclave.activate(0);
      await expect(
        enclave.publishInput(0, "0xaabbcc"),
      ).to.be.revertedWithCustomError(enclave, "InvalidInput");
    });

    it("reverts if outside of input window", async function () {
      const { enclave, request } = await loadFixture(setup);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      await enclave.activate(0);
      await expect(enclave.publishInput(0, ZeroHash)).to.not.be.reverted;

      await mine(2, { interval: request.duration });

      await expect(
        enclave.publishInput(0, ZeroHash),
      ).to.be.revertedWithCustomError(enclave, "InputDeadlinePassed");
    });
    it("sets ciphertextInput correctly", async function () {
      const { enclave, request } = await loadFixture(setup);
      const inputData = "0x12345678";

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      await enclave.activate(0);

      expect(await enclave.publishInput(0, inputData)).to.not.be.reverted;
      let e3 = await enclave.getE3(0);
      expect(e3.inputs[0]).to.equal(inputData);
      expect(await enclave.publishInput(0, inputData)).to.not.be.reverted;
      e3 = await enclave.getE3(0);
      expect(e3.inputs[1]).to.equal(inputData);
    });
    it("returns true if input is published successfully", async function () {
      const { enclave, request } = await loadFixture(setup);
      const inputData = "0x12345678";

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      await enclave.activate(0);

      expect(await enclave.publishInput.staticCall(0, inputData)).to.equal(
        true,
      );
    });
    it("emits InputPublished event", async function () {
      const { enclave, request } = await loadFixture(setup);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );

      const e3Id = 0;

      const inputData = abiCoder.encode(["bytes"], ["0xaabbccddeeff"]);
      await enclave.activate(e3Id);

      await expect(enclave.publishInput(e3Id, inputData))
        .to.emit(enclave, "InputPublished")
        .withArgs(e3Id, inputData);
    });
  });

  describe("publishCiphertextOutput()", function () {
    it("reverts if E3 does not exist", async function () {
      const { enclave, request } = await loadFixture(setup);

      await expect(enclave.publishCiphertextOutput(0, "0x"))
        .to.be.revertedWithCustomError(enclave, "E3DoesNotExist")
        .withArgs(0);
    });
    it("reverts if E3 has not been activated", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await expect(enclave.publishCiphertextOutput(e3Id, "0x"))
        .to.be.revertedWithCustomError(enclave, "E3NotActivated")
        .withArgs(e3Id);
    });
    it("reverts if input deadline has not passed", async function () {
      const { enclave, request } = await loadFixture(setup);
      const tx = await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      const block = await tx.getBlock();
      const timestamp = block ? block.timestamp : await time.latest();
      const expectedExpiration = timestamp + request.duration + 1;
      const e3Id = 0;

      await enclave.activate(e3Id);

      await expect(enclave.publishCiphertextOutput(e3Id, "0x"))
        .to.be.revertedWithCustomError(enclave, "InputDeadlineNotPassed")
        .withArgs(e3Id, expectedExpiration);
    });
    it("reverts if output has already been published", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await enclave.activate(e3Id);
      await mine(2, { interval: request.duration });
      expect(await enclave.publishCiphertextOutput(e3Id, "0x1337"));
      await expect(enclave.publishCiphertextOutput(e3Id, "0x1337"))
        .to.be.revertedWithCustomError(
          enclave,
          "CiphertextOutputAlreadyPublished",
        )
        .withArgs(e3Id);
    });
    it("reverts if output is not valid", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await enclave.activate(e3Id);
      await mine(2, { interval: request.duration });
      await expect(
        enclave.publishCiphertextOutput(e3Id, "0x"),
      ).to.be.revertedWithCustomError(enclave, "InvalidOutput");
    });
    it("sets ciphertextOutput correctly", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await enclave.activate(e3Id);
      await mine(2, { interval: request.duration });
      expect(await enclave.publishCiphertextOutput(e3Id, "0x1337"));
      const e3 = await enclave.getE3(e3Id);
      expect(e3.ciphertextOutput).to.equal("0x1337");
    });
    it("returns true if output is published successfully", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await enclave.activate(e3Id);
      await mine(2, { interval: request.duration });
      expect(
        await enclave.publishCiphertextOutput.staticCall(e3Id, "0x1337"),
      ).to.equal(true);
    });
    it("emits CiphertextOutputPublished event", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await enclave.activate(e3Id);
      await mine(2, { interval: request.duration });
      await expect(enclave.publishCiphertextOutput(e3Id, "0x1337"))
        .to.emit(enclave, "CiphertextOutputPublished")
        .withArgs(e3Id, "0x1337");
    });
  });

  describe("publishPlaintextOutput()", function () {
    it("reverts if E3 does not exist", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await expect(enclave.publishPlaintextOutput(e3Id, "0x"))
        .to.be.revertedWithCustomError(enclave, "E3DoesNotExist")
        .withArgs(e3Id);
    });
    it("reverts if E3 has not been activated", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await expect(enclave.publishPlaintextOutput(e3Id, "0x"))
        .to.be.revertedWithCustomError(enclave, "E3NotActivated")
        .withArgs(e3Id);
    });
    it("reverts if ciphertextOutput has not been published", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await enclave.activate(e3Id);
      await expect(enclave.publishPlaintextOutput(e3Id, "0x"))
        .to.be.revertedWithCustomError(enclave, "CiphertextOutputNotPublished")
        .withArgs(e3Id);
    });
    it("reverts if plaintextOutput has already been published", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await enclave.activate(e3Id);
      await mine(2, { interval: request.duration });
      await enclave.publishCiphertextOutput(e3Id, "0x1337");
      await enclave.publishPlaintextOutput(e3Id, "0x1337");
      await expect(enclave.publishPlaintextOutput(e3Id, "0x1337"))
        .to.be.revertedWithCustomError(
          enclave,
          "PlaintextOutputAlreadyPublished",
        )
        .withArgs(e3Id);
    });
    it("reverts if output is not valid", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await enclave.activate(e3Id);
      await mine(2, { interval: request.duration });
      await enclave.publishCiphertextOutput(e3Id, "0x1337");
      await expect(enclave.publishPlaintextOutput(e3Id, "0x"))
        .to.be.revertedWithCustomError(enclave, "InvalidOutput")
        .withArgs("0x");
    });
    it("sets plaintextOutput correctly", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await enclave.activate(e3Id);
      await mine(2, { interval: request.duration });
      await enclave.publishCiphertextOutput(e3Id, "0x1337");
      expect(await enclave.publishPlaintextOutput(e3Id, "0x1337"));

      const e3 = await enclave.getE3(e3Id);
      expect(e3.plaintextOutput).to.equal("0x1337");
    });
    it("returns true if output is published successfully", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await enclave.activate(e3Id);
      await mine(2, { interval: request.duration });
      await enclave.publishCiphertextOutput(e3Id, "0x1337");
      expect(
        await enclave.publishPlaintextOutput.staticCall(e3Id, "0x1337"),
      ).to.equal(true);
    });
    it("emits PlaintextOutputPublished event", async function () {
      const { enclave, request } = await loadFixture(setup);
      const e3Id = 0;

      await enclave.request(
        request.filter,
        request.threshold,
        [await time.latest(), (await time.latest()) + 100],
        request.duration,
        request.computationModule,
        request.cMParams,
        request.executionModule,
        request.eMParams,
        { value: 10 },
      );
      await enclave.activate(e3Id);
      await mine(2, { interval: request.duration });
      await enclave.publishCiphertextOutput(e3Id, "0x1337");
      expect(await enclave.publishPlaintextOutput(e3Id, "0x1337"))
        .to.emit(enclave, "PlaintextOutputPublished")
        .withArgs(e3Id, "0x1337");
    });
  });
});
