// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { LeanIMT } from "@zk-kit/lean-imt";
import { expect } from "chai";
import { network } from "hardhat";
import { poseidon2 } from "poseidon-lite";

import CiphernodeRegistryModule from "../../ignition/modules/ciphernodeRegistry";
import NaiveRegistryFilterModule from "../../ignition/modules/naiveRegistryFilter";
import {
  CiphernodeRegistryOwnable__factory as CiphernodeRegistryFactory,
  NaiveRegistryFilter__factory as NaiveRegistryFilterFactory,
} from "../../types";

const AddressOne = "0x0000000000000000000000000000000000000001";
const AddressTwo = "0x0000000000000000000000000000000000000002";
const AddressThree = "0x0000000000000000000000000000000000000003";

const { ethers, networkHelpers, ignition } = await network.connect();
const { loadFixture } = networkHelpers;

const data = "0xda7a";
const dataHash = ethers.keccak256(data);

// Hash function used to compute the tree nodes.
const hash = (a: bigint, b: bigint) => poseidon2([a, b]);

describe("CiphernodeRegistryOwnable", function () {
  async function setup() {
    const [owner, notTheOwner] = await ethers.getSigners();

    const registryContract = await ignition.deploy(CiphernodeRegistryModule, {
      parameters: {
        CiphernodeRegistry: {
          enclaveAddress: await owner.getAddress(),
          owner: await owner.getAddress(),
        },
      },
    });

    const filterContract = await ignition.deploy(NaiveRegistryFilterModule, {
      parameters: {
        NaiveRegistryFilter: {
          owner: await owner.getAddress(),
          ciphernodeRegistryAddress:
            await registryContract.cipherNodeRegistry.getAddress(),
        },
      },
    });

    const registry = CiphernodeRegistryFactory.connect(
      await registryContract.cipherNodeRegistry.getAddress(),
      owner,
    );
    const filter = NaiveRegistryFilterFactory.connect(
      await filterContract.naiveRegistryFilter.getAddress(),
      owner,
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
    it("correctly sets `_owner` and `enclave` ", async function () {
      const poseidonFactory = await ethers.getContractFactory("PoseidonT3");
      const poseidonDeployment = await poseidonFactory.deploy();
      const [deployer] = await ethers.getSigners();
      if (!deployer) throw new Error("Bad getSigners() output");
      const ciphernodeRegistryFactory = await ethers.getContractFactory(
        "CiphernodeRegistryOwnable",
        {
          libraries: {
            PoseidonT3: await poseidonDeployment.getAddress(),
          },
        },
      );
      const ciphernodeRegistry = await ciphernodeRegistryFactory.deploy(
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

      await filter.setRegistry(await owner.getAddress());
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
        registry.publishCommittee(request.e3Id, "0xc0de", data),
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
        data,
      );
      expect(await registry.committeePublicKey(request.e3Id)).to.equal(
        dataHash,
      );
    });
    it("emits a CommitteePublished event", async function () {
      const { filter, registry, request } = await loadFixture(setup);
      await registry.requestCommittee(
        request.e3Id,
        request.filter,
        request.threshold,
      );
      await expect(
        await filter.publishCommittee(
          request.e3Id,
          [AddressOne, AddressTwo],
          data,
        ),
      )
        .to.emit(registry, "CommitteePublished")
        .withArgs(request.e3Id, data);
    });
  });

  describe("addCiphernode()", function () {
    it("reverts if the caller is not the owner", async function () {
      const { registry, notTheOwner } = await loadFixture(setup);
      await expect(
        registry.connect(notTheOwner).addCiphernode(AddressThree),
      ).to.be.revertedWithCustomError(registry, "NotOwnerOrBondingRegistry");
    });
    it("adds the ciphernode to the registry", async function () {
      const { registry } = await loadFixture(setup);
      expect(await registry.addCiphernode(AddressThree));
      expect(await registry.isEnabled(AddressThree)).to.be.true;
    });
    it("increments numCiphernodes", async function () {
      const { registry } = await loadFixture(setup);
      const numCiphernodes = await registry.numCiphernodes();
      expect(await registry.addCiphernode(AddressThree));
      expect(await registry.numCiphernodes()).to.equal(
        numCiphernodes + BigInt(1),
      );
    });
    it("emits a CiphernodeAdded event", async function () {
      const { registry } = await loadFixture(setup);
      const treeSize = await registry.treeSize();
      const numCiphernodes = await registry.numCiphernodes();
      await expect(await registry.addCiphernode(AddressThree))
        .to.emit(registry, "CiphernodeAdded")
        .withArgs(
          AddressThree,
          treeSize,
          numCiphernodes + BigInt(1),
          treeSize + BigInt(1),
        );
    });
  });

  describe("removeCiphernode()", function () {
    it("reverts if the caller is not the owner", async function () {
      const { registry, notTheOwner } = await loadFixture(setup);
      await expect(
        registry.connect(notTheOwner).removeCiphernode(AddressOne, []),
      ).to.be.revertedWithCustomError(registry, "NotOwnerOrBondingRegistry");
    });
    it("removes the ciphernode from the registry", async function () {
      const { registry } = await loadFixture(setup);
      const tree = new LeanIMT(hash);
      tree.insert(BigInt(AddressOne));
      tree.insert(BigInt(AddressTwo));
      const index = tree.indexOf(BigInt(AddressOne));
      const proof = tree.generateProof(index);
      tree.update(index, BigInt(0));
      expect(await registry.isEnabled(AddressOne)).to.be.true;
      expect(await registry.removeCiphernode(AddressOne, proof.siblings));
      expect(await registry.isEnabled(AddressOne)).to.be.false;
      expect(await registry.root()).to.equal(tree.root);
    });
    it("decrements numCiphernodes", async function () {
      const { registry, tree } = await loadFixture(setup);
      const numCiphernodes = await registry.numCiphernodes();
      const index = tree.indexOf(BigInt(AddressOne));
      const proof = tree.generateProof(index);
      expect(await registry.removeCiphernode(AddressOne, proof.siblings));
      expect(await registry.numCiphernodes()).to.equal(
        numCiphernodes - BigInt(1),
      );
    });
    it("emits a CiphernodeRemoved event", async function () {
      const { registry, tree } = await loadFixture(setup);
      const numCiphernodes = await registry.numCiphernodes();
      const size = await registry.treeSize();
      const index = tree.indexOf(BigInt(AddressOne));
      const proof = tree.generateProof(index);
      await expect(registry.removeCiphernode(AddressOne, proof.siblings))
        .to.emit(registry, "CiphernodeRemoved")
        .withArgs(AddressOne, index, numCiphernodes - BigInt(1), size);
    });
  });

  describe("setEnclave()", function () {
    it("reverts if the caller is not the owner", async function () {
      const { registry, notTheOwner } = await loadFixture(setup);
      await expect(
        registry.connect(notTheOwner).setEnclave(AddressThree),
      ).to.be.revertedWithCustomError(registry, "OwnableUnauthorizedAccount");
    });
    it("sets the enclave address", async function () {
      const { registry } = await loadFixture(setup);
      expect(await registry.setEnclave(AddressThree));
      expect(await registry.enclave()).to.equal(AddressThree);
    });
    it("emits an EnclaveSet event", async function () {
      const { registry } = await loadFixture(setup);
      await expect(await registry.setEnclave(AddressThree))
        .to.emit(registry, "EnclaveSet")
        .withArgs(AddressThree);
    });
  });

  describe("committeePublicKey()", function () {
    it("returns the public key of the committee for the given e3Id", async function () {
      const { filter, registry, request } = await loadFixture(setup);
      await registry.requestCommittee(
        request.e3Id,
        request.filter,
        request.threshold,
      );
      await filter.publishCommittee(
        request.e3Id,
        [AddressOne, AddressTwo],
        data,
      );
      expect(await registry.committeePublicKey(request.e3Id)).to.equal(
        dataHash,
      );
    });
    it("reverts if the committee has not been published", async function () {
      const { registry, request } = await loadFixture(setup);
      await registry.requestCommittee(
        request.e3Id,
        request.filter,
        request.threshold,
      );
      await expect(
        registry.committeePublicKey(request.e3Id),
      ).to.be.revertedWithCustomError(registry, "CommitteeNotPublished");
    });
  });

  describe("isCiphernodeEligible()", function () {
    it("returns true if the ciphernode is in the registry", async function () {
      const { registry } = await loadFixture(setup);
      expect(await registry.isEnabled(AddressOne)).to.be.true;
    });
    it("returns false if the ciphernode is not in the registry", async function () {
      const { registry } = await loadFixture(setup);
      expect(await registry.isCiphernodeEligible(AddressThree)).to.be.false;
    });
  });

  describe("isEnabled()", function () {
    it("returns true if the ciphernode is currently enabled", async function () {
      const { registry } = await loadFixture(setup);
      expect(await registry.isEnabled(AddressOne)).to.be.true;
    });
    it("returns false if the ciphernode is not currently enabled", async function () {
      const { registry } = await loadFixture(setup);
      expect(await registry.isEnabled(AddressThree)).to.be.false;
    });
  });

  describe("root()", function () {
    it("returns the root of the ciphernode registry merkle tree", async function () {
      const { registry, tree } = await loadFixture(setup);
      expect(await registry.root()).to.equal(tree.root);
    });
  });

  describe("rootAt()", function () {
    it("returns the root of the ciphernode registry merkle tree at the given e3Id", async function () {
      const { registry, tree, request } = await loadFixture(setup);
      await registry.requestCommittee(
        request.e3Id,
        request.filter,
        request.threshold,
      );
      expect(await registry.rootAt(request.e3Id)).to.equal(tree.root);
    });
  });

  describe("getFilter()", function () {
    it("returns the registry filter for the given e3Id", async function () {
      const { registry, request } = await loadFixture(setup);
      await registry.requestCommittee(
        request.e3Id,
        request.filter,
        request.threshold,
      );
      expect(await registry.getFilter(request.e3Id)).to.equal(request.filter);
    });
  });

  describe("treeSize()", function () {
    it("returns the size of the ciphernode registry merkle tree", async function () {
      const { registry, tree } = await loadFixture(setup);
      expect(await registry.treeSize()).to.equal(tree.size);
    });
  });
});
