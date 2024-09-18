import {
  loadFixture,
  mine,
  time,
} from "@nomicfoundation/hardhat-network-helpers";
import { LeanIMT } from "@zk-kit/lean-imt";
import { expect } from "chai";
import { ZeroHash } from "ethers";
import { ethers, network } from "hardhat";
import { poseidon2 } from "poseidon-lite";

import { deployCiphernodeRegistryOwnableFixture } from "../fixtures/CiphernodeRegistryOwnable.fixture";
import { naiveRegistryFilterFixture } from "../fixtures/NaiveRegistryFilter.fixture";
import { PoseidonT3Fixture } from "../fixtures/PoseidonT3.fixture";

const abiCoder = ethers.AbiCoder.defaultAbiCoder();
const AddressOne = "0x0000000000000000000000000000000000000001";
const AddressTwo = "0x0000000000000000000000000000000000000002";
const addressThree = "0x0000000000000000000000000000000000000003";

// Hash function used to compute the tree nodes.
const hash = (a: bigint, b: bigint) => poseidon2([a, b]);

describe.only("CiphernodeRegistryOwnable", function () {
  async function setup() {
    const [owner, notTheOwner] = await ethers.getSigners();

    const poseidon = await PoseidonT3Fixture();
    const registry = await deployCiphernodeRegistryOwnableFixture(
      owner.address,
      owner.address,
      await poseidon.getAddress(),
    );
    const filter = await naiveRegistryFilterFixture(
      owner.address,
      await registry.getAddress(),
    );
    await registry.addCiphernode(AddressOne);
    await registry.addCiphernode(AddressTwo);

    return {
      owner,
      notTheOwner,
      registry,
      filter,
      request: {
        e3Id: 1,
        filter: await filter.getAddress(),
        threshold: [2, 2] as [number, number],
      },
    };
  }

  describe("constructor / initialize()", function () {
    it("correctly sets `_owner` and `enclave` ", async function () {
      const poseidonFactory = await ethers.getContractFactory("PoseidonT3");
      const poseidonDeployment = await poseidonFactory.deploy();
      const [deployer] = await ethers.getSigners();
      let ciphernodeRegistryFactory = await ethers.getContractFactory(
        "CiphernodeRegistryOwnable",
        {
          libraries: {
            PoseidonT3: await poseidonDeployment.getAddress(),
          },
        },
      );
      let ciphernodeRegistry = await ciphernodeRegistryFactory.deploy(
        deployer.address,
        AddressTwo,
      );
      expect(await ciphernodeRegistry.owner()).to.equal(deployer.address);
      expect(await ciphernodeRegistry.enclave()).to.equal(AddressTwo);
    });
  });

  describe("requestCommittee()", function () {
    it("reverts if committee has already been requested for given e3Id", async function () {
      const { registry, request } = await loadFixture(setup);
      await registry.requestCommittee(
        request.e3Id,
        request.filter,
        request.threshold,
      );
      await expect(
        registry.requestCommittee(
          request.e3Id,
          request.filter,
          request.threshold,
        ),
      ).to.be.revertedWithCustomError(registry, "CommitteeAlreadyRequested");
    });
    it("stores the registry filter for the given e3Id", async function () {
      const { registry, request } = await loadFixture(setup);
      await registry.requestCommittee(
        request.e3Id,
        request.filter,
        request.threshold,
      );
      expect(await registry.getFilter(request.e3Id)).to.equal(request.filter);
    });
    it("stores the root of the ciphernode registry at the time of the request", async function () {
      const { registry, request } = await loadFixture(setup);
      await registry.requestCommittee(
        request.e3Id,
        request.filter,
        request.threshold,
      );
      expect(await registry.rootAt(request.e3Id)).to.equal(
        await registry.root(),
      );
    });
    it("requests a committee from the given filter", async function () {
      const { registry, request } = await loadFixture(setup);
      await registry.requestCommittee(
        request.e3Id,
        request.filter,
        request.threshold,
      );
      expect(await registry.getFilter(request.e3Id)).to.equal(request.filter);
    });
    it("emits a CommitteeRequested event", async function () {
      const { registry, request } = await loadFixture(setup);
      await expect(
        registry.requestCommittee(
          request.e3Id,
          request.filter,
          request.threshold,
        ),
      )
        .to.emit(registry, "CommitteeRequested")
        .withArgs(request.e3Id, request.filter, request.threshold);
    });
    it("reverts if filter.requestCommittee() fails", async function () {
      const { owner, registry, filter, request } = await loadFixture(setup);

      await filter.setRegistry(owner.address);
      await filter.requestCommittee(request.e3Id, request.threshold);
      await filter.setRegistry(await registry.getAddress());

      await expect(
        registry.requestCommittee(
          request.e3Id,
          request.filter,
          request.threshold,
        ),
      ).to.be.revertedWithCustomError(filter, "CommitteeAlreadyExists");
    });
    it("returns true if the request is successful", async function () {
      const { registry, request } = await loadFixture(setup);
      expect(
        await registry.requestCommittee.staticCall(
          request.e3Id,
          request.filter,
          request.threshold,
        ),
      ).to.be.true;
    });
  });

  describe("publishCommittee()", function () {
    it("reverts if the caller is not the filter for the given e3Id", async function () {
      const { registry, request } = await loadFixture(setup);
      await registry.requestCommittee(
        request.e3Id,
        request.filter,
        request.threshold,
      );
      await expect(
        registry.publishCommittee(request.e3Id, "0xc0de", "0xda7a"),
      ).to.be.revertedWithCustomError(registry, "OnlyFilter");
    });
    it("stores the public key of the committee", async function () {
      const { filter, registry, request } = await loadFixture(setup);
      await registry.requestCommittee(
        request.e3Id,
        request.filter,
        request.threshold,
      );
      await filter.publishCommittee(
        request.e3Id,
        [AddressOne, AddressTwo],
        "0xda7a",
      );
      expect(await registry.committeePublicKey(request.e3Id)).to.equal(
        "0xda7a",
      );
    });
    it("emits a CommitteePublished event", async function () {
      const { filter, registry, request } = await loadFixture(setup);
      await registry.requestCommittee(
        request.e3Id,
        request.filter,
        request.threshold,
      );
      expect(
        await filter.publishCommittee(
          request.e3Id,
          [AddressOne, AddressTwo],
          "0xda7a",
        ),
      )
        .to.emit(registry, "CommitteePublished")
        .withArgs(request.e3Id, "0xda7a");
    });
  });

  describe("addCiphernode()", function () {
    it("reverts if the caller is not the owner", async function () {
      const { registry, notTheOwner } = await loadFixture(setup);
      await expect(registry.connect(notTheOwner).addCiphernode(addressThree))
        .to.be.revertedWithCustomError(registry, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner.address);
    });
    it("adds the ciphernode to the registry", async function () {
      const { registry } = await loadFixture(setup);
      expect(await registry.addCiphernode(addressThree));
      expect(await registry.isCiphernodeEligible(addressThree)).to.be.true;
    });
    it("increments numCiphernodes", async function () {
      const { registry } = await loadFixture(setup);
      const numCiphernodes = await registry.numCiphernodes();
      expect(await registry.addCiphernode(addressThree));
      expect(await registry.numCiphernodes()).to.equal(
        numCiphernodes + BigInt(1),
      );
    });
    it("emits a CiphernodeAdded event", async function () {
      const { registry } = await loadFixture(setup);
      const numCiphernodes = await registry.numCiphernodes();
      expect(await registry.addCiphernode(addressThree))
        .to.emit(registry, "CiphernodeAdded")
        .withArgs(addressThree, numCiphernodes + BigInt(1));
    });
  });

  describe("removeCiphernode()", function () {
    it("reverts if the caller is not the owner");
    it("removes the ciphernode from the registry");
    it("decrements numCiphernodes");
    it("emits a CiphernodeRemoved event");
  });

  describe("setEnclave()", function () {
    it("reverts if the caller is not the owner");
    it("sets the enclave address");
    it("emits an EnclaveSet event");
  });

  describe("committeePublicKey()", function () {
    it("returns the public key of the committee for the given e3Id");
    it("reverts if the committee has not been published");
  });

  describe("isCiphernodeEligible()", function () {
    it("returns true if the ciphernode is in the registry");
    it("returns false if the ciphernode is not in the registry");
  });

  describe("isEnabled()", function () {
    it("returns true if the ciphernode is currently enabled");
    it("returns false if the ciphernode is not currently enabled");
  });

  describe("root()", function () {
    it("returns the root of the ciphernode registry merkle tree");
  });

  describe("rootAt()", function () {
    it(
      "returns the root of the ciphernode registry merkle tree at the given e3Id",
    );
  });

  describe("getFilter()", function () {
    it("returns the registry filter for the given e3Id");
  });
});
