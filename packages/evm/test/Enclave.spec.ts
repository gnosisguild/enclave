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
import { deployInputValidatorPolicyFixture } from "./fixtures/InputValidatorPolicy.fixture";
import { deployCiphernodeRegistryFixture } from "./fixtures/MockCiphernodeRegistry.fixture";
import { deployComputeProviderFixture } from "./fixtures/MockComputeProvider.fixture";
import { deployDecryptionVerifierFixture } from "./fixtures/MockDecryptionVerifier.fixture";
import { deployE3ProgramFixture } from "./fixtures/MockE3Program.fixture";
import { deployInputValidatorCheckerFixture } from "./fixtures/MockInputValidatorChecker.fixture";
import { PoseidonT3Fixture } from "./fixtures/PoseidonT3.fixture";

const abiCoder = ethers.AbiCoder.defaultAbiCoder();
const AddressTwo = "0x0000000000000000000000000000000000000002";
const AddressSix = "0x0000000000000000000000000000000000000006";
const encryptionSchemeId =
  "0x2c2a814a0495f913a3a312fc4771e37552bc14f8a2d4075a08122d356f0849c6";
const newEncryptionSchemeId =
  "0x0000000000000000000000000000000000000000000000000000000000000002";

const FilterFail = AddressTwo;
const FilterOkay = AddressSix;

const data = "0xda7a";
const dataHash = ethers.keccak256(data);
const _publicKeyHash = ethers.keccak256(abiCoder.encode(["uint256"], [0]));
const proof = "0x1337";

// Hash function used to compute the tree nodes.
const hash = (a: bigint, b: bigint) => poseidon2([a, b]);

async function deployInputValidatorContracts() {
  const inputValidatorChecker = await deployInputValidatorCheckerFixture();
  const inputValidatorPolicy = await deployInputValidatorPolicyFixture(
    await inputValidatorChecker.getAddress(),
  );

  return {
    inputValidatorChecker,
    inputValidatorPolicy,
  };
}

describe("Enclave", function () {
  async function setup() {
    const [owner, notTheOwner] = await ethers.getSigners();

    const poseidon = await PoseidonT3Fixture();
    const registry = await deployCiphernodeRegistryFixture();
    const decryptionVerifier = await deployDecryptionVerifierFixture();
    const computeProvider = await deployComputeProviderFixture();

    const { inputValidatorPolicy } = await deployInputValidatorContracts();

    const e3Program = await deployE3ProgramFixture(
      await inputValidatorPolicy.getAddress(),
    );

    const enclave = await deployEnclaveFixture(
      owner.address,
      await registry.getAddress(),
      await poseidon.getAddress(),
    );

    // Ensure we set the target of the calling contract
    await inputValidatorPolicy.connect(owner).setTarget(enclave);

    await enclave.setDecryptionVerifier(
      encryptionSchemeId,
      await decryptionVerifier.getAddress(),
    );

    await enclave.enableE3Program(await e3Program.getAddress());

    return {
      owner,
      notTheOwner,
      enclave,
      poseidon,
      mocks: {
        e3Program,
        decryptionVerifier,
        computeProvider,
        inputValidatorPolicy,
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
        e3ProgramParams: "0x12345678",
        computeProviderParams: abiCoder.encode(
          ["address"],
          [await decryptionVerifier.getAddress()],
        ),
      },
    };
  }

  describe("constructor / initialize()", function () {
    it("correctly sets owner", async function () {
      const { owner, enclave } = await loadFixture(setup);
      expect(await enclave.owner()).to.equal(owner.address);
    });

    it("correctly sets ciphernodeRegistry address", async function () {
      const { mocks, enclave } = await loadFixture(setup);
      expect(await enclave.ciphernodeRegistry()).to.equal(
        await mocks.registry.getAddress(),
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
      const { enclave, mocks, request } = await loadFixture(setup);
      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.e3Program,
        request.e3ProgramParams,
        request.computeProviderParams,
        { value: 10 },
      );
      const e3 = await enclave.getE3(0);

      expect(e3.threshold).to.deep.equal(request.threshold);
      expect(e3.expiration).to.equal(0n);
      expect(e3.e3Program).to.equal(request.e3Program);
      expect(e3.e3ProgramParams).to.equal(request.e3ProgramParams);
      expect(e3.inputValidator).to.equal(
        await mocks.inputValidatorPolicy.getAddress(),
      );
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
      const { enclave, notTheOwner, mocks } = await loadFixture(setup);

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
    it("returns true if decryption verifier is enabled successfully", async function () {
      const { enclave, mocks } = await loadFixture(setup);

      const result = await enclave.setDecryptionVerifier.staticCall(
        newEncryptionSchemeId,
        await mocks.decryptionVerifier.getAddress(),
      );
      expect(result).to.be.true;
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
    it("returns true if encryption scheme is disabled successfully", async function () {
      const { enclave } = await loadFixture(setup);

      const result =
        await enclave.disableEncryptionScheme.staticCall(encryptionSchemeId);
      expect(result).to.be.true;
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
          request.computeProviderParams,
          { value: 10 },
        ),
      )
        .to.be.revertedWithCustomError(enclave, "E3ProgramNotAllowed")
        .withArgs(ethers.ZeroAddress);
    });
    it("reverts if given encryption scheme is not enabled", async function () {
      const { enclave, request } = await loadFixture(setup);
      await enclave.disableEncryptionScheme(encryptionSchemeId);
      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          request.duration,
          request.e3Program,
          abiCoder.encode(["bytes", "address"], [ZeroHash, ethers.ZeroAddress]),
          request.computeProviderParams,
          { value: 10 },
        ),
      )
        .to.be.revertedWithCustomError(enclave, "InvalidEncryptionScheme")
        .withArgs(encryptionSchemeId);
    });
    it("reverts if given E3 Program does not return input validator address", async function () {
      const { enclave, mocks, owner, request } = await loadFixture(setup);
      await mocks.e3Program
        .connect(owner)
        .setInputValidator(ethers.ZeroAddress);
      await expect(
        enclave.request(
          request.filter,
          request.threshold,
          request.startTime,
          request.duration,
          request.e3Program,
          request.e3ProgramParams,
          request.computeProviderParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(enclave, "InvalidComputationRequest");
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
          request.computeProviderParams,
          { value: 10 },
        ),
      ).to.be.revertedWithCustomError(enclave, "CommitteeSelectionFailed");
    });
    it("instantiates a new E3", async function () {
      const { enclave, mocks, request } = await loadFixture(setup);
      await enclave.request(
        request.filter,
        request.threshold,
        request.startTime,
        request.duration,
        request.e3Program,
        request.e3ProgramParams,
        request.computeProviderParams,
        { value: 10 },
      );
      const e3 = await enclave.getE3(0);
      const block = await ethers.provider.getBlock("latest").catch((e) => e);

      expect(e3.threshold).to.deep.equal(request.threshold);
      expect(e3.expiration).to.equal(0n);
      expect(e3.e3Program).to.equal(request.e3Program);
      expect(e3.requestBlock).to.equal(block.number);
      expect(e3.inputValidator).to.equal(
        await mocks.inputValidatorPolicy.getAddress(),
      );
      expect(e3.decryptionVerifier).to.equal(
        abiCoder.decode(["address"], request.computeProviderParams)[0],
      );
      expect(e3.committeePublicKey).to.equal(ethers.ZeroHash);
      expect(e3.ciphertextOutput).to.equal(ethers.ZeroHash);
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
        request.computeProviderParams,
        { value: 10 },
      );
      const e3 = await enclave.getE3(0);

      await expect(tx)
        .to.emit(enclave, "E3Requested")
        .withArgs(0, e3, request.filter, request.e3Program);
    });
  });

  describe("activate()", function () {
    it("reverts if E3 does not exist", async function () {
      const { enclave } = await loadFixture(setup);

      await expect(enclave.activate(0, ethers.ZeroHash))
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
        request.computeProviderParams,
        { value: 10 },
      );

      await expect(enclave.getE3(0)).to.not.be.reverted;
      await expect(enclave.activate(0, ethers.ZeroHash)).to.not.be.reverted;
      await expect(enclave.activate(0, ethers.ZeroHash))
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
        request.computeProviderParams,
        { value: 10 },
      );

      await expect(
        enclave.activate(0, ethers.ZeroHash),
      ).to.be.revertedWithCustomError(enclave, "E3NotReady");
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
        request.computeProviderParams,
        { value: 10 },
      );

      await mine(2, { interval: 2000 });

      await expect(
        enclave.activate(0, ethers.ZeroHash),
      ).to.be.revertedWithCustomError(enclave, "E3Expired");
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
        request.computeProviderParams,
        { value: 10 },
      );

      await expect(
        enclave.activate(0, ethers.ZeroHash),
      ).to.be.revertedWithCustomError(enclave, "E3NotReady");
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
        request.computeProviderParams,
        { value: 10 },
      );

      await mine(1, { interval: 1000 });

      await expect(
        enclave.activate(0, ethers.ZeroHash),
      ).to.be.revertedWithCustomError(enclave, "E3Expired");
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
        request.computeProviderParams,
        { value: 10 },
      );

      const prevRegistry = await enclave.ciphernodeRegistry();
      const nextRegistry = await deployCiphernodeRegistryFixture(
        "MockCiphernodeRegistryEmptyKey",
      );

      await enclave.setCiphernodeRegistry(nextRegistry);
      await expect(
        enclave.activate(0, ethers.ZeroHash),
      ).to.be.revertedWithCustomError(enclave, "CommitteeSelectionFailed");

      await enclave.setCiphernodeRegistry(prevRegistry);
      await expect(enclave.activate(0, ethers.ZeroHash)).to.not.be.reverted;
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
        request.computeProviderParams,
        { value: 10 },
      );

      const e3Id = 0;
      const publicKey = await registry.committeePublicKey(e3Id);

      let e3 = await enclave.getE3(e3Id);
      expect(e3.committeePublicKey).to.not.equal(ethers.keccak256(publicKey));

      await enclave.activate(e3Id, ethers.ZeroHash);

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
        request.computeProviderParams,
        { value: 10 },
      );

      const e3Id = 0;

      expect(
        await enclave.activate.staticCall(e3Id, ethers.ZeroHash),
      ).to.be.equal(true);
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
        request.computeProviderParams,
        { value: 10 },
      );

      const e3Id = 0;
      const e3 = await enclave.getE3(e3Id);

      await expect(enclave.activate(e3Id, ethers.ZeroHash))
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
        request.computeProviderParams,
        { value: 10 },
      );

      const inputData = abiCoder.encode(["bytes32"], [ZeroHash]);

      await expect(enclave.getE3(0)).to.not.be.reverted;
      await expect(enclave.publishInput(0, inputData))
        .to.be.revertedWithCustomError(enclave, "E3NotActivated")
        .withArgs(0);

      await enclave.activate(0, ethers.ZeroHash);
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
        request.computeProviderParams,
        { value: 10 },
      );

      await enclave.activate(0, ethers.ZeroHash);
      await expect(
        enclave.publishInput(0, "0xaabbcc"),
      ).to.be.revertedWithCustomError(enclave, "UnsuccessfulCheck");
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
        request.computeProviderParams,
        { value: 10 },
      );

      await enclave.activate(0, ethers.ZeroHash);

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
        request.computeProviderParams,
        { value: 10 },
      );

      await enclave.activate(0, ethers.ZeroHash);

      expect(await enclave.publishInput.staticCall(0, inputData)).to.equal(
        true,
      );
    });

    // Skipping for now as fixing this would mean implementing an AdvancedPolicy in excubiae
    it.skip("adds inputHash to merkle tree", async function () {
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
        request.computeProviderParams,
        { value: 10 },
      );

      const e3Id = 0;

      await enclave.activate(e3Id, ethers.ZeroHash);

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
        request.computeProviderParams,
        { value: 10 },
      );

      const e3Id = 0;

      const inputData = abiCoder.encode(["bytes"], ["0xaabbccddeeff"]);
      await enclave.activate(e3Id, ethers.ZeroHash);
      const expectedHash = hash(BigInt(ethers.keccak256(inputData)), BigInt(0));

      await expect(enclave.publishInput(e3Id, inputData))
        .to.emit(enclave, "InputPublished")
        .withArgs(e3Id, inputData, expectedHash, 0);
    });
  });

  describe("publishCiphertextOutput()", function () {
    it("reverts if E3 does not exist", async function () {
      const { enclave } = await loadFixture(setup);

      await expect(enclave.publishCiphertextOutput(0, "0x", "0x"))
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
        request.computeProviderParams,
        { value: 10 },
      );
      await expect(enclave.publishCiphertextOutput(e3Id, "0x", "0x"))
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
        request.computeProviderParams,
        { value: 10 },
      );
      const block = await tx.getBlock();
      const timestamp = block ? block.timestamp : await time.latest();
      const expectedExpiration = timestamp + request.duration + 1;
      const e3Id = 0;

      await enclave.activate(e3Id, ethers.ZeroHash);

      await expect(enclave.publishCiphertextOutput(e3Id, "0x", "0x"))
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
        request.computeProviderParams,
        { value: 10 },
      );
      await enclave.activate(e3Id, ethers.ZeroHash);
      await mine(2, { interval: request.duration });
      expect(await enclave.publishCiphertextOutput(e3Id, data, proof));
      await expect(enclave.publishCiphertextOutput(e3Id, data, proof))
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
        request.computeProviderParams,
        { value: 10 },
      );
      await enclave.activate(e3Id, ethers.ZeroHash);
      await mine(2, { interval: request.duration });
      await expect(
        enclave.publishCiphertextOutput(e3Id, "0x", "0x"),
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
        request.computeProviderParams,
        { value: 10 },
      );
      await enclave.activate(e3Id, ethers.ZeroHash);
      await mine(2, { interval: request.duration });
      expect(await enclave.publishCiphertextOutput(e3Id, data, proof));
      const e3 = await enclave.getE3(e3Id);
      expect(e3.ciphertextOutput).to.equal(dataHash);
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
        request.computeProviderParams,
        { value: 10 },
      );
      await enclave.activate(e3Id, ethers.ZeroHash);
      await mine(2, { interval: request.duration });
      expect(
        await enclave.publishCiphertextOutput.staticCall(e3Id, data, proof),
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
        request.computeProviderParams,
        { value: 10 },
      );
      await enclave.activate(e3Id, ethers.ZeroHash);
      await mine(2, { interval: request.duration });
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
        request.computeProviderParams,
        { value: 10 },
      );
      await expect(enclave.publishPlaintextOutput(e3Id, data, "0x"))
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
        request.computeProviderParams,
        { value: 10 },
      );
      await enclave.activate(e3Id, ethers.ZeroHash);
      await expect(enclave.publishPlaintextOutput(e3Id, data, "0x"))
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
        request.computeProviderParams,
        { value: 10 },
      );
      await enclave.activate(e3Id, ethers.ZeroHash);
      await mine(2, { interval: request.duration });
      await enclave.publishCiphertextOutput(e3Id, data, proof);
      await enclave.publishPlaintextOutput(e3Id, data, proof);
      await expect(enclave.publishPlaintextOutput(e3Id, data, proof))
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
        request.computeProviderParams,
        { value: 10 },
      );
      await enclave.activate(e3Id, ethers.ZeroHash);
      await mine(2, { interval: request.duration });
      await enclave.publishCiphertextOutput(e3Id, data, proof);
      await expect(enclave.publishPlaintextOutput(e3Id, data, "0x"))
        .to.be.revertedWithCustomError(enclave, "InvalidOutput")
        .withArgs(data);
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
        request.computeProviderParams,
        { value: 10 },
      );
      await enclave.activate(e3Id, ethers.ZeroHash);
      await mine(2, { interval: request.duration });
      await enclave.publishCiphertextOutput(e3Id, data, proof);
      expect(await enclave.publishPlaintextOutput(e3Id, data, proof));

      const e3 = await enclave.getE3(e3Id);
      expect(e3.plaintextOutput).to.equal(data);
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
        request.computeProviderParams,
        { value: 10 },
      );
      await enclave.activate(e3Id, ethers.ZeroHash);
      await mine(2, { interval: request.duration });
      await enclave.publishCiphertextOutput(e3Id, data, proof);
      expect(
        await enclave.publishPlaintextOutput.staticCall(e3Id, data, proof),
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
        request.computeProviderParams,
        { value: 10 },
      );
      await enclave.activate(e3Id, ethers.ZeroHash);
      await mine(2, { interval: request.duration });
      await enclave.publishCiphertextOutput(e3Id, data, proof);
      await expect(await enclave.publishPlaintextOutput(e3Id, data, proof))
        .to.emit(enclave, "PlaintextOutputPublished")
        .withArgs(e3Id, data);
    });
  });
});

describe("InputValidatorPolicy", function () {
  it("should validate inputs using the input validator", async function () {
    const [owner, notTheOwner] = await ethers.getSigners();
    const { inputValidatorPolicy } = await loadFixture(
      deployInputValidatorContracts,
    );
    await inputValidatorPolicy.connect(owner).setTarget(owner);
    const shouldPass = "0x1234"; // length 2 = pass
    const contract = inputValidatorPolicy.connect(owner);
    await contract.connect(owner).enforce(notTheOwner, [shouldPass]);
    await expect(
      contract.connect(owner).enforce(notTheOwner, [shouldPass]),
    ).to.be.revertedWithCustomError(inputValidatorPolicy, "AlreadyEnforced");
  });

  it("should fail with error if the checker fails", async function () {
    const [owner, notTheOwner] = await ethers.getSigners();

    const { inputValidatorPolicy } = await loadFixture(
      deployInputValidatorContracts,
    );
    await inputValidatorPolicy.connect(owner).setTarget(owner);
    const shouldFail = "0x123456"; // length 3 = fail
    const contract = inputValidatorPolicy.connect(owner);
    await expect(
      contract.enforce(notTheOwner, [shouldFail]),
    ).to.be.revertedWithCustomError(contract, "UnsuccessfulCheck");
  });
});
