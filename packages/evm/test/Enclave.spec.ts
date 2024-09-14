import {
  loadFixture,
  mine,
  time,
} from "@nomicfoundation/hardhat-network-helpers";
import { LeanIMT } from "@zk-kit/lean-imt";
import { expect } from "chai";
import { ZeroHash } from "ethers";
import { ethers } from "hardhat";
import { poseidon2 } from "poseidon-lite";

import { deployEnclaveFixture } from "./fixtures/Enclave.fixture";
import { deployCiphernodeRegistryFixture } from "./fixtures/MockCiphernodeRegistry.fixture";
import { deployComputeProviderFixture } from "./fixtures/MockComputeProvider.fixture";
import { deployDecryptionVerifierFixture } from "./fixtures/MockDecryptionVerifier.fixture";
import { deployE3ProgramFixture } from "./fixtures/MockE3Program.fixture";
import { deployInputValidatorFixture } from "./fixtures/MockInputValidator.fixture";

const abiCoder = ethers.AbiCoder.defaultAbiCoder();
const AddressTwo = "0x0000000000000000000000000000000000000002";
const AddressSix = "0x0000000000000000000000000000000000000006";

const FilterFail = AddressTwo;
const FilterOkay = AddressSix;

// Hash function used to compute the tree nodes.
const hash = (a: bigint, b: bigint) => poseidon2([a, b]);

describe("Enclave", function () {
  async function setup() {
    const [owner, notTheOwner] = await ethers.getSigners();

    const registry = await deployCiphernodeRegistryFixture();
    const e3Program = await deployE3ProgramFixture();
    const decryptionVerifier = await deployDecryptionVerifierFixture();
    const computeProvider = await deployComputeProviderFixture();
    const inputValidator = await deployInputValidatorFixture();

    const enclave = await deployEnclaveFixture({
      owner,
      registry: await registry.getAddress(),
    });

    await enclave.enableE3Program(await e3Program.getAddress());
    await enclave.enableComputeProvider(await computeProvider.getAddress());

    return {
      owner,
      notTheOwner,
      enclave,
      mocks: {
        e3Program,
        decryptionVerifier,
        computeProvider,
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
        e3Program: await e3Program.getAddress(),
        e3ProgramParams: abiCoder.encode(
          ["address"],
          [await inputValidator.getAddress()],
        ),
        computeProvider: await computeProvider.getAddress(),
        computeProviderParams: abiCoder.encode(
          ["address"],
          [await decryptionVerifier.getAddress()],
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
      expect(await enclave.ciphernodeRegistry()).to.equal(AddressTwo);
    });

    it("correctly sets ciphernodeRegistry address", async function () {
      const [aSigner] = await ethers.getSigners();
      const enclave = await deployEnclaveFixture({
        owner: aSigner,
        registry: AddressTwo,
      });
      expect(await enclave.ciphernodeRegistry()).to.equal(AddressTwo);
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
      const {
        enclave,
        mocks: { registry },
      } = await loadFixture(setup);
      await expect(enclave.setCiphernodeRegistry(registry))
        .to.be.revertedWithCustomError(enclave, "InvalidCiphernodeRegistry")
        .withArgs(registry);
    });
    it("sets ciphernodeRegistry correctly", async function () {
      const { enclave } = await loadFixture(setup);

      expect(await enclave.ciphernodeRegistry()).to.not.equal(AddressTwo);
      await enclave.setCiphernodeRegistry(AddressTwo);
      expect(await enclave.ciphernodeRegistry()).to.equal(AddressTwo);
    });
    it("returns true if ciphernodeRegistry is set successfully", async function () {
      const { enclave } = await loadFixture(setup);

      const result = await enclave.setCiphernodeRegistry.staticCall(AddressTwo);
      expect(result).to.be.true;
    });
    it("emits CiphernodeRegistrySet event", async function () {
      const { enclave } = await loadFixture(setup);

      await expect(enclave.setCiphernodeRegistry(AddressTwo))
        .to.emit(enclave, "CiphernodeRegistrySet")
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );
      const e3 = await enclave.getE3(0);

      expect(e3.threshold).to.deep.equal(request.threshold);
      expect(e3.expiration).to.equal(0n);
      expect(e3.e3Program).to.equal(request.e3Program);
      expect(e3.e3ProgramParams).to.equal(request.e3ProgramParams);
      expect(e3.inputValidator).to.equal(
        abiCoder.decode(["address"], request.e3ProgramParams)[0],
      );
      expect(e3.computeProvider).to.equal(request.computeProvider);
      expect(e3.decryptionVerifier).to.equal(
        abiCoder.decode(["address"], request.computeProviderParams)[0],
      );
      expect(e3.committeePublicKey).to.equal("0x");
      expect(e3.ciphertextOutput).to.equal("0x");
      expect(e3.plaintextOutput).to.equal("0x");
    });
  });

  describe("enableE3Program()", function () {
    it("reverts if not called by owner", async function () {
      const {
        notTheOwner,
        enclave,
        mocks: { e3Program },
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
    it("returns true if E3 Program is enabled successfully", async function () {
      const { enclave } = await loadFixture(setup);
      const result = await enclave.enableE3Program.staticCall(AddressTwo);
      expect(result).to.be.true;
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
        notTheOwner,
        enclave,
        mocks: { e3Program },
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
    it("returns true if E3 Program is disabled successfully", async function () {
      const {
        enclave,
        mocks: { e3Program },
      } = await loadFixture(setup);
      const result = await enclave.disableE3Program.staticCall(e3Program);

      expect(result).to.be.true;
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

  describe("enableComputeProvider()", function () {
    it("reverts if not called by owner", async function () {
      const { notTheOwner, enclave } = await loadFixture(setup);
      await expect(
        enclave.connect(notTheOwner).enableComputeProvider(AddressTwo),
      )
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner.address);
    });
    it("reverts if compute provider is already enabled", async function () {
      const {
        enclave,
        mocks: { computeProvider },
      } = await loadFixture(setup);
      await expect(enclave.enableComputeProvider(computeProvider))
        .to.be.revertedWithCustomError(enclave, "ModuleAlreadyEnabled")
        .withArgs(computeProvider);
    });
    it("enables compute provider correctly", async function () {
      const {
        enclave,
        mocks: { computeProvider },
      } = await loadFixture(setup);
      const enabled = await enclave.computeProviders(computeProvider);
      expect(enabled).to.be.true;
    });
    it("returns true if compute provider is enabled successfully", async function () {
      const { enclave } = await loadFixture(setup);
      const result = await enclave.enableComputeProvider.staticCall(AddressTwo);

      expect(result).to.be.true;
    });
    it("emits ComputeProviderEnabled event", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(enclave.enableComputeProvider(AddressTwo))
        .to.emit(enclave, "ComputeProviderEnabled")
        .withArgs(AddressTwo);
    });
  });

  describe("disableComputeProvider()", function () {
    it("reverts if not called by owner", async function () {
      const {
        notTheOwner,
        enclave,
        mocks: { computeProvider },
      } = await loadFixture(setup);

      await expect(
        enclave.connect(notTheOwner).disableComputeProvider(computeProvider),
      )
        .to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner);
    });
    it("reverts if compute provider is not enabled", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(enclave.disableComputeProvider(AddressTwo))
        .to.be.revertedWithCustomError(enclave, "ModuleNotEnabled")
        .withArgs(AddressTwo);
    });
    it("disables compute provider correctly", async function () {
      const {
        enclave,
        mocks: { computeProvider },
      } = await loadFixture(setup);

      expect(await enclave.computeProviders(computeProvider)).to.be.true;
      await enclave.disableComputeProvider(computeProvider);
      expect(await enclave.computeProviders(computeProvider)).to.be.false;
    });
    it("returns true if compute provider is disabled successfully", async function () {
      const {
        enclave,
        mocks: { computeProvider },
      } = await loadFixture(setup);
      const result =
        await enclave.disableComputeProvider.staticCall(computeProvider);

      expect(result).to.be.true;
    });
    it("emits ComputeProviderDisabled event", async function () {
      const {
        enclave,
        mocks: { computeProvider },
      } = await loadFixture(setup);

      await expect(enclave.disableComputeProvider(computeProvider))
        .to.emit(enclave, "ComputeProviderDisabled")
        .withArgs(computeProvider);
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
          request.e3Program,
          request.e3ProgramParams,
          request.computeProvider,
          request.computeProviderParams,
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
          request.e3Program,
          request.e3ProgramParams,
          request.computeProvider,
          request.computeProviderParams,
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
          request.e3Program,
          request.e3ProgramParams,
          request.computeProvider,
          request.computeProviderParams,
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
          request.e3Program,
          request.e3ProgramParams,
          request.computeProvider,
          request.computeProviderParams,
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
          request.e3Program,
          request.e3ProgramParams,
          request.computeProvider,
          request.computeProviderParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(enclave, "InvalidDuration");
    });
    it("reverts if E3 Program is not enabled", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          request.duration,
          ethers.ZeroAddress,
          request.e3ProgramParams,
          request.computeProvider,
          request.computeProviderParams,
          { value: 10 },
        ),
      )
        .to.be.revertedWithCustomError(enclave, "E3ProgramNotAllowed")
        .withArgs(ethers.ZeroAddress);
    });
    it("reverts if compute provider is not enabled", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          request.duration,
          request.e3Program,
          request.e3ProgramParams,
          ethers.ZeroAddress,
          request.computeProviderParams,
          { value: 10 },
        ),
      )
        .to.be.revertedWithCustomError(enclave, "ModuleNotEnabled")
        .withArgs(ethers.ZeroAddress);
    });
    it("reverts if input E3 Program does not return input validator address", async function () {
      const { enclave, request } = await loadFixture(setup);

      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          request.duration,
          request.e3Program,
          ZeroHash,
          request.computeProvider,
          request.computeProviderParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(enclave, "InvalidComputation");
    });
    it("reverts if input compute provider does not return output verifier address", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          request.duration,
          request.e3Program,
          request.e3ProgramParams,
          request.computeProvider,
          ZeroHash,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(enclave, "InvalidComputeProviderSetup");
    });
    it("reverts if committee selection fails", async function () {
      const { enclave, request } = await loadFixture(setup);
      await expect(
        enclave.request(
          FilterFail,
          request.threshold,
          request.startTime,
          request.duration,
          request.e3Program,
          request.e3ProgramParams,
          request.computeProvider,
          request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );
      const e3 = await enclave.getE3(0);

      expect(e3.threshold).to.deep.equal(request.threshold);
      expect(e3.expiration).to.equal(0n);
      expect(e3.e3Program).to.equal(request.e3Program);
      expect(e3.inputValidator).to.equal(
        abiCoder.decode(["address"], request.e3ProgramParams)[0],
      );
      expect(e3.computeProvider).to.equal(request.computeProvider);
      expect(e3.decryptionVerifier).to.equal(
        abiCoder.decode(["address"], request.computeProviderParams)[0],
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );
      const e3 = await enclave.getE3(0);

      await expect(tx)
        .to.emit(enclave, "E3Requested")
        .withArgs(
          0,
          e3,
          request.filter,
          request.e3Program,
          request.computeProvider,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );

      await expect(enclave.activate(0)).to.be.revertedWithCustomError(
        enclave,
        "E3NotReady",
      );
    });
    it("reverts if E3 start has expired", async function () {
      const { enclave, request } = await loadFixture(setup);
      const startTime = [
        (await time.latest()) + 1,
        (await time.latest()) + 1000,
      ] as [number, number];

      await enclave.request(
        request.filter,
        request.threshold,
        startTime,
        request.duration,
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );

      await mine(2, { interval: 2000 });

      await expect(enclave.activate(0)).to.be.revertedWithCustomError(
        enclave,
        "E3Expired",
      );
    });
    it("reverts if ciphernodeRegistry does not return a public key", async function () {
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );

      await mine(1, { interval: 1000 });

      await expect(enclave.activate(0)).to.be.revertedWithCustomError(
        enclave,
        "E3Expired",
      );
    });
    it("reverts if ciphernodeRegistry does not return a public key", async function () {
      const { enclave, request } = await loadFixture(setup);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );

      const prevRegistry = await enclave.ciphernodeRegistry();
      const nextRegistry = await deployCiphernodeRegistryFixture(
        "MockCiphernodeRegistryEmptyKey",
      );

      await enclave.setCiphernodeRegistry(nextRegistry);
      await expect(enclave.activate(0)).to.be.revertedWithCustomError(
        enclave,
        "CommitteeSelectionFailed",
      );

      await enclave.setCiphernodeRegistry(prevRegistry);
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );

      const inputData = abiCoder.encode(["bytes32"], [ZeroHash]);

      await expect(enclave.getE3(0)).to.not.be.reverted;
      await expect(enclave.publishInput(0, inputData))
        .to.be.revertedWithCustomError(enclave, "E3NotActivated")
        .withArgs(0);

      await enclave.activate(0);
    });

    it("reverts if input is not valid", async function () {
      const { enclave, request } = await loadFixture(setup);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );

      await enclave.activate(0);

      await mine(2, { interval: request.duration });

      await expect(
        enclave.publishInput(0, ZeroHash),
      ).to.be.revertedWithCustomError(enclave, "InputDeadlinePassed");
    });
    it("returns true if input is published successfully", async function () {
      const { enclave, request } = await loadFixture(setup);
      const inputData = "0x12345678";

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );

      await enclave.activate(0);

      expect(await enclave.publishInput.staticCall(0, inputData)).to.equal(
        true,
      );
    });

    it("adds inputHash to merkle tree", async function () {
      const { enclave, request } = await loadFixture(setup);
      const inputData = abiCoder.encode(["bytes"], ["0xaabbccddeeff"]);

      // To create an instance of a LeanIMT, you must provide the hash function.
      const tree = new LeanIMT(hash);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );

      const e3Id = 0;

      await enclave.activate(e3Id);

      tree.insert(hash(BigInt(ethers.keccak256(inputData)), BigInt(0)));

      await enclave.publishInput(e3Id, inputData);
      expect(await enclave.getInputRoot(e3Id)).to.equal(tree.root);

      const secondInputData = abiCoder.encode(["bytes"], ["0x112233445566"]);
      tree.insert(hash(BigInt(ethers.keccak256(secondInputData)), BigInt(1)));
      await enclave.publishInput(e3Id, secondInputData);
      expect(await enclave.getInputRoot(e3Id)).to.equal(tree.root);
    });
    it("emits InputPublished event", async function () {
      const { enclave, request } = await loadFixture(setup);

      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );

      const e3Id = 0;

      const inputData = abiCoder.encode(["bytes"], ["0xaabbccddeeff"]);
      await enclave.activate(e3Id);
      const expectedHash = hash(BigInt(ethers.keccak256(inputData)), BigInt(0));

      await expect(enclave.publishInput(e3Id, inputData))
        .to.emit(enclave, "InputPublished")
        .withArgs(e3Id, inputData, expectedHash, 0);
    });
  });

  describe("publishCiphertextOutput()", function () {
    it("reverts if E3 does not exist", async function () {
      const { enclave } = await loadFixture(setup);

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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
      const { enclave } = await loadFixture(setup);
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
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
        request.e3Program,
        request.e3ProgramParams,
        request.computeProvider,
        request.computeProviderParams,
        { value: 10 },
      );
      await enclave.activate(e3Id);
      await mine(2, { interval: request.duration });
      await enclave.publishCiphertextOutput(e3Id, "0x1337");
      await expect(await enclave.publishPlaintextOutput(e3Id, "0x1337"))
        .to.emit(enclave, "PlaintextOutputPublished")
        .withArgs(e3Id, "0x1337");
    });
  });
});
