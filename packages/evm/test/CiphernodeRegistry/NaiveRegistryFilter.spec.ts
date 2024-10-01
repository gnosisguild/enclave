import { loadFixture } from "@nomicfoundation/hardhat-network-helpers";
import { LeanIMT } from "@zk-kit/lean-imt";
import { expect } from "chai";
import { ethers } from "hardhat";
import { poseidon2 } from "poseidon-lite";

import { deployCiphernodeRegistryOwnableFixture } from "../fixtures/CiphernodeRegistryOwnable.fixture";
import { naiveRegistryFilterFixture } from "../fixtures/NaiveRegistryFilter.fixture";
import { PoseidonT3Fixture } from "../fixtures/PoseidonT3.fixture";

const AddressOne = "0x0000000000000000000000000000000000000001";
const AddressTwo = "0x0000000000000000000000000000000000000002";
const AddressThree = "0x0000000000000000000000000000000000000003";

// Hash function used to compute the tree nodes.
const hash = (a: bigint, b: bigint) => poseidon2([a, b]);

describe("NaiveRegistryFilter", function () {
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

    const tree = new LeanIMT(hash);
    await registry.addCiphernode(AddressOne);
    tree.insert(BigInt(AddressOne));
    await registry.addCiphernode(AddressTwo);
    tree.insert(BigInt(AddressTwo));

    return {
      owner,
      notTheOwner,
      registry,
      filter,
      tree,
      request: {
        e3Id: 1,
        filter: await filter.getAddress(),
        threshold: [2, 2] as [number, number],
      },
    };
  }

  describe("constructor / initialize()", function () {
    it("should set the owner", async function () {
      const { owner, filter } = await loadFixture(setup);
      expect(await filter.owner()).to.equal(owner.address);
    });
    it("should set the registry", async function () {
      const { registry, filter } = await loadFixture(setup);
      expect(await filter.registry()).to.equal(await registry.getAddress());
    });
  });

  describe("requestCommittee()", function () {
    it("should revert if the caller is not the registry", async function () {
      const { notTheOwner, filter, request } = await loadFixture(setup);
      await expect(
        filter
          .connect(notTheOwner)
          .requestCommittee(request.e3Id, request.threshold),
      ).to.be.revertedWithCustomError(filter, "OnlyRegistry");
    });
    it("should revert if a committee has already been requested for the given e3Id", async function () {
      const { filter, request, owner } = await loadFixture(setup);
      await filter.setRegistry(owner.address);
      await filter.requestCommittee(request.e3Id, request.threshold);
      await expect(
        filter.requestCommittee(request.e3Id, request.threshold),
      ).to.be.revertedWithCustomError(filter, "CommitteeAlreadyExists");
    });
    it("should set the threshold for the requested committee", async function () {
      const { filter, owner, request } = await loadFixture(setup);
      await filter.setRegistry(owner.address);
      await filter.requestCommittee(request.e3Id, request.threshold);
      const committee = await filter.getCommittee(request.e3Id);
      expect(committee.threshold).to.deep.equal(request.threshold);
    });
    it("should return true when a committee is requested", async function () {
      const { filter, owner, request } = await loadFixture(setup);
      await filter.setRegistry(owner.address);
      const result = await filter.requestCommittee.staticCall(
        request.e3Id,
        request.threshold,
      );
      expect(result).to.equal(true);
    });
  });

  describe("publishCommittee()", function () {
    it("should revert if the caller is not owner", async function () {
      const { filter, notTheOwner, request } = await loadFixture(setup);
      await expect(
        filter
          .connect(notTheOwner)
          .publishCommittee(
            request.e3Id,
            [AddressOne, AddressTwo],
            AddressThree,
          ),
      ).to.be.revertedWithCustomError(filter, "OwnableUnauthorizedAccount");
    });
    it("should revert if committee already published", async function () {
      const { filter, registry, request } = await loadFixture(setup);
      expect(
        await registry.requestCommittee(
          request.e3Id,
          request.filter,
          request.threshold,
        ),
      );
      await filter.publishCommittee(
        request.e3Id,
        [AddressOne, AddressTwo],
        AddressThree,
      );
      await expect(
        filter.publishCommittee(
          request.e3Id,
          [AddressOne, AddressTwo],
          AddressThree,
        ),
      ).to.be.revertedWithCustomError(filter, "CommitteeAlreadyPublished");
    });
    it("should store the node addresses of the committee", async function () {
      const { filter, registry, request } = await loadFixture(setup);
      expect(
        await registry.requestCommittee(
          request.e3Id,
          request.filter,
          request.threshold,
        ),
      );
      await filter.publishCommittee(
        request.e3Id,
        [AddressOne, AddressTwo],
        AddressThree,
      );
      const committee = await filter.getCommittee(request.e3Id);
      expect(committee.nodes).to.deep.equal([AddressOne, AddressTwo]);
    });
    it("should store the public key of the committee", async function () {
      const { filter, registry, request } = await loadFixture(setup);
      expect(
        await registry.requestCommittee(
          request.e3Id,
          request.filter,
          request.threshold,
        ),
      );
      await filter.publishCommittee(
        request.e3Id,
        [AddressOne, AddressTwo],
        AddressThree,
      );
      const committee = await filter.getCommittee(request.e3Id);
      expect(committee.publicKey).to.equal(ethers.keccak256(AddressThree));
    });
    it("should publish the correct node addresses of the committee for the given e3Id", async function () {
      const { filter, registry, request } = await loadFixture(setup);
      expect(
        await registry.requestCommittee(
          request.e3Id,
          request.filter,
          request.threshold,
        ),
      );
      await filter.publishCommittee(
        request.e3Id,
        [AddressOne, AddressTwo],
        AddressThree,
      );
      const committee = await filter.getCommittee(request.e3Id);
      expect(committee.nodes).to.deep.equal([AddressOne, AddressTwo]);
    });
    it("should publish the public key of the committee for the given e3Id", async function () {
      const { filter, registry, request } = await loadFixture(setup);
      expect(
        await registry.requestCommittee(
          request.e3Id,
          request.filter,
          request.threshold,
        ),
      );
      await filter.publishCommittee(
        request.e3Id,
        [AddressOne, AddressTwo],
        AddressThree,
      );
      const committee = await filter.getCommittee(request.e3Id);
      expect(committee.publicKey).to.equal(ethers.keccak256(AddressThree));
    });
  });

  describe("setRegistry()", function () {
    it("should revert if the caller is not the owner", async function () {
      const { filter, notTheOwner } = await loadFixture(setup);
      await expect(filter.connect(notTheOwner).setRegistry(notTheOwner.address))
        .to.be.revertedWithCustomError(filter, "OwnableUnauthorizedAccount")
        .withArgs(notTheOwner.address);
    });
    it("should set the registry", async function () {
      const { filter, owner } = await loadFixture(setup);
      await filter.setRegistry(owner.address);
      expect(await filter.registry()).to.equal(owner.address);
    });
  });

  describe("getCommittee()", function () {
    it("should return the committee for the given e3Id", async function () {
      const { filter, registry, request } = await loadFixture(setup);
      expect(
        await registry.requestCommittee(
          request.e3Id,
          request.filter,
          request.threshold,
        ),
      );
      expect(
        await filter.publishCommittee(
          request.e3Id,
          [AddressOne, AddressTwo],
          AddressThree,
        ),
      );
      const committee = await filter.getCommittee(request.e3Id);
      expect(committee.threshold).to.deep.equal(request.threshold);
      expect(committee.nodes).to.deep.equal([AddressOne, AddressTwo]);
      expect(committee.publicKey).to.equal(ethers.keccak256(AddressThree));
    });
  });
});
