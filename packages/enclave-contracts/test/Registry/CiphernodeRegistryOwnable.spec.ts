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

import BondingRegistryModule from "../../ignition/modules/bondingRegistry";
import CiphernodeRegistryModule from "../../ignition/modules/ciphernodeRegistry";
import EnclaveTicketTokenModule from "../../ignition/modules/enclaveTicketToken";
import EnclaveTokenModule from "../../ignition/modules/enclaveToken";
import MockStableTokenModule from "../../ignition/modules/mockStableToken";
import SlashingManagerModule from "../../ignition/modules/slashingManager";
import {
  BondingRegistry__factory as BondingRegistryFactory,
  CiphernodeRegistryOwnable__factory as CiphernodeRegistryFactory,
} from "../../types";

const AddressOne = "0x0000000000000000000000000000000000000001";
const AddressTwo = "0x0000000000000000000000000000000000000002";
const AddressThree = "0x0000000000000000000000000000000000000003";

const { ethers, networkHelpers, ignition } = await network.connect();
const { loadFixture } = networkHelpers;

const data = "0xda7a";
const dataHash = ethers.keccak256(data);
const SORTITION_SUBMISSION_WINDOW = 3;

// Hash function used to compute the tree nodes.
const hash = (a: bigint, b: bigint) => poseidon2([a, b]);

describe("CiphernodeRegistryOwnable", function () {
  async function finalizeCommitteeAfterWindow(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    registry: any,
    e3Id: number,
  ): Promise<void> {
    await networkHelpers.time.increase(SORTITION_SUBMISSION_WINDOW + 1);
    await registry.finalizeCommittee(e3Id);
  }

  async function setupOperatorForSortition(
    operator: Signer,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    bondingRegistry: any,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    licenseToken: any,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    usdcToken: any,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    ticketToken: any,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
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

  async function setup() {
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
            baseToken: await usdcContract.mockUSDC.getAddress(),
            registry: AddressOne,
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
            bondingRegistry: AddressOne,
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
            registry: AddressOne,
            slashedFundsTreasury: ownerAddress,
            ticketPrice: ethers.parseUnits("10", 6),
            licenseRequiredBond: ethers.parseEther("1000"),
            minTicketBalance: 5,
            exitDelay: 7 * 24 * 60 * 60,
          },
        },
      },
    );

    const registryContract = await ignition.deploy(CiphernodeRegistryModule, {
      parameters: {
        CiphernodeRegistry: {
          enclaveAddress: ownerAddress,
          owner: ownerAddress,
          submissionWindow: SORTITION_SUBMISSION_WINDOW,
        },
      },
    });

    const registryAddress =
      await registryContract.cipherNodeRegistry.getAddress();

    const registry = CiphernodeRegistryFactory.connect(registryAddress, owner);

    const bondingRegistry = BondingRegistryFactory.connect(
      await bondingRegistryContract.bondingRegistry.getAddress(),
      owner,
    );

    await ticketTokenContract.enclaveTicketToken.setRegistry(
      await bondingRegistry.getAddress(),
    );
    await bondingRegistry.setRegistry(registryAddress);
    await bondingRegistry.setSlashingManager(
      await slashingManagerContract.slashingManager.getAddress(),
    );
    await slashingManagerContract.slashingManager.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );

    await registry.setBondingRegistry(await bondingRegistry.getAddress());

    const tree = new LeanIMT(hash);
    const licenseToken = enclTokenContract.enclaveToken;
    const ticketToken = ticketTokenContract.enclaveTicketToken;
    const usdcToken = usdcContract.mockUSDC;

    await licenseToken.setTransferRestriction(false);
    await setupOperatorForSortition(
      operator1,
      bondingRegistry,
      licenseToken,
      usdcToken,
      ticketToken,
      registry,
    );
    tree.insert(BigInt(await operator1.getAddress()));

    await setupOperatorForSortition(
      operator2,
      bondingRegistry,
      licenseToken,
      usdcToken,
      ticketToken,
      registry,
    );
    tree.insert(BigInt(await operator2.getAddress()));
    await networkHelpers.mine(1);

    return {
      owner,
      notTheOwner,
      operator1,
      operator2,
      registry,
      bondingRegistry,
      licenseToken,
      ticketToken,
      usdcToken,
      tree,
      request: {
        e3Id: 1,
        threshold: [2, 2] as [number, number],
      },
    };
  }

  describe("constructor / initialize()", function () {
    it("correctly sets `_owner` and `enclave` ", async function () {
      const poseidonFactory = await ethers.getContractFactory("PoseidonT3");
      const poseidonDeployment = await poseidonFactory.deploy();
      await poseidonDeployment.waitForDeployment();
      const poseidonAddress = await poseidonDeployment.getAddress();
      const [deployer] = await ethers.getSigners();
      if (!deployer) throw new Error("Bad getSigners() output");

      const ciphernodeRegistryFactory = await ethers.getContractFactory(
        "CiphernodeRegistryOwnable",
        {
          libraries: {
            PoseidonT3: poseidonAddress,
          },
        },
      );
      const implementation = await ciphernodeRegistryFactory.deploy();
      await implementation.waitForDeployment();
      const implementationAddress = await implementation.getAddress();

      const initData = ciphernodeRegistryFactory.interface.encodeFunctionData(
        "initialize",
        [deployer.address, AddressTwo, SORTITION_SUBMISSION_WINDOW],
      );

      const proxyFactory = await ethers.getContractFactory(
        "TransparentUpgradeableProxy",
      );
      const proxy = await proxyFactory.deploy(
        implementationAddress,
        deployer.address,
        initData,
      );
      await proxy.waitForDeployment();
      const proxyAddress = await proxy.getAddress();

      const ciphernodeRegistry = CiphernodeRegistryFactory.connect(
        proxyAddress,
        deployer,
      );

      expect(await ciphernodeRegistry.owner()).to.equal(deployer.address);
      expect(await ciphernodeRegistry.enclave()).to.equal(AddressTwo);
      expect(await ciphernodeRegistry.sortitionSubmissionWindow()).to.equal(
        SORTITION_SUBMISSION_WINDOW,
      );
    });
  });

  describe("requestCommittee()", function () {
    it("reverts if committee has already been requested for given e3Id", async function () {
      const { registry, request } = await loadFixture(setup);
      await registry.requestCommittee(request.e3Id, 0, request.threshold);
      await expect(
        registry.requestCommittee(request.e3Id, 0, request.threshold),
      ).to.be.revertedWithCustomError(registry, "CommitteeAlreadyRequested");
    });
    it("stores the root of the ciphernode registry at the time of the request", async function () {
      const { registry, request } = await loadFixture(setup);
      await registry.requestCommittee(request.e3Id, 0, request.threshold);
      expect(await registry.rootAt(request.e3Id)).to.equal(
        await registry.root(),
      );
    });
    it("emits a CommitteeRequested event", async function () {
      const { registry, request } = await loadFixture(setup);

      const tx = await registry.requestCommittee(
        request.e3Id,
        0n,
        request.threshold,
      );
      const receipt = await tx.wait();
      if (!receipt) throw new Error("Transaction failed");

      const sWindow = await registry.sortitionSubmissionWindow();
      const block = await ethers.provider.getBlock(receipt.blockNumber);
      if (!block) throw new Error("Block not found");

      const expectedBlockNumber = BigInt(receipt.blockNumber);
      const expectedDeadline = BigInt(block.timestamp) + sWindow;

      await expect(tx)
        .to.emit(registry, "CommitteeRequested")
        .withArgs(
          request.e3Id,
          0n,
          request.threshold,
          expectedBlockNumber,
          expectedDeadline,
        );
    });
    it("returns true if the request is successful", async function () {
      const { registry, request } = await loadFixture(setup);
      expect(
        await registry.requestCommittee.staticCall(
          request.e3Id,
          0,
          request.threshold,
        ),
      ).to.be.true;
    });
  });

  describe("publishCommittee()", function () {
    it("reverts if the caller is not the owner", async function () {
      const { registry, request, notTheOwner, operator1, operator2 } =
        await loadFixture(setup);
      await registry.requestCommittee(request.e3Id, 0, request.threshold);

      await registry.connect(operator1).submitTicket(request.e3Id, 1);
      await registry.connect(operator2).submitTicket(request.e3Id, 1);
      await finalizeCommitteeAfterWindow(registry, request.e3Id);

      await expect(
        registry
          .connect(notTheOwner)
          .publishCommittee(
            request.e3Id,
            [await operator1.getAddress(), await operator2.getAddress()],
            data,
          ),
      ).to.be.revertedWithCustomError(registry, "OwnableUnauthorizedAccount");
    });
    it("stores the public key of the committee", async function () {
      const { registry, request, operator1, operator2 } =
        await loadFixture(setup);
      await registry.requestCommittee(request.e3Id, 0, request.threshold);

      await networkHelpers.mine(1);

      await registry.connect(operator1).submitTicket(request.e3Id, 1);
      await registry.connect(operator2).submitTicket(request.e3Id, 1);
      await finalizeCommitteeAfterWindow(registry, request.e3Id);

      await registry.publishCommittee(
        request.e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
      );
      expect(await registry.committeePublicKey(request.e3Id)).to.equal(
        dataHash,
      );
    });
    it("emits a CommitteePublished event", async function () {
      const { registry, request, operator1, operator2 } =
        await loadFixture(setup);
      await registry.requestCommittee(request.e3Id, 0, request.threshold);

      // Submit tickets from both operators and finalize
      await registry.connect(operator1).submitTicket(request.e3Id, 1);
      await registry.connect(operator2).submitTicket(request.e3Id, 1);
      await finalizeCommitteeAfterWindow(registry, request.e3Id);

      await expect(
        await registry.publishCommittee(
          request.e3Id,
          [await operator1.getAddress(), await operator2.getAddress()],
          data,
        ),
      )
        .to.emit(registry, "CommitteePublished")
        .withArgs(
          request.e3Id,
          [await operator1.getAddress(), await operator2.getAddress()],
          data,
        );
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
      const { registry, operator1, tree } = await loadFixture(setup);
      const operator1Address = await operator1.getAddress();
      const localTree = new LeanIMT(hash);
      for (let i = 0; i < tree.size; i++) {
        localTree.insert(tree.leaves[i]);
      }
      const index = localTree.indexOf(BigInt(operator1Address));
      const proof = localTree.generateProof(index);
      localTree.update(index, BigInt(0));
      expect(await registry.isEnabled(operator1Address)).to.be.true;
      expect(await registry.removeCiphernode(operator1Address, proof.siblings));
      expect(await registry.isEnabled(operator1Address)).to.be.false;
      expect(await registry.root()).to.equal(localTree.root);
    });
    it("decrements numCiphernodes", async function () {
      const { registry, operator1, tree } = await loadFixture(setup);
      const operator1Address = await operator1.getAddress();
      const numCiphernodes = await registry.numCiphernodes();
      const index = tree.indexOf(BigInt(operator1Address));
      const proof = tree.generateProof(index);
      expect(await registry.removeCiphernode(operator1Address, proof.siblings));
      expect(await registry.numCiphernodes()).to.equal(
        numCiphernodes - BigInt(1),
      );
    });
    it("emits a CiphernodeRemoved event", async function () {
      const { registry, operator1, tree } = await loadFixture(setup);
      const operator1Address = await operator1.getAddress();
      const numCiphernodes = await registry.numCiphernodes();
      const size = await registry.treeSize();
      const index = tree.indexOf(BigInt(operator1Address));
      const proof = tree.generateProof(index);
      await expect(registry.removeCiphernode(operator1Address, proof.siblings))
        .to.emit(registry, "CiphernodeRemoved")
        .withArgs(operator1Address, index, numCiphernodes - BigInt(1), size);
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
      const { registry, request, operator1, operator2 } =
        await loadFixture(setup);
      await registry.requestCommittee(request.e3Id, 0, request.threshold);

      await registry.connect(operator1).submitTicket(request.e3Id, 1);
      await registry.connect(operator2).submitTicket(request.e3Id, 1);
      await finalizeCommitteeAfterWindow(registry, request.e3Id);

      await registry.publishCommittee(
        request.e3Id,
        [await operator1.getAddress(), await operator2.getAddress()],
        data,
      );
      expect(await registry.committeePublicKey(request.e3Id)).to.equal(
        dataHash,
      );
    });
    it("reverts if the committee has not been published", async function () {
      const { registry, request } = await loadFixture(setup);
      await registry.requestCommittee(request.e3Id, 0, request.threshold);
      await expect(
        registry.committeePublicKey(request.e3Id),
      ).to.be.revertedWithCustomError(registry, "CommitteeNotPublished");
    });
  });

  describe("isCiphernodeEligible()", function () {
    it("returns true if the ciphernode is in the registry", async function () {
      const { registry, operator1 } = await loadFixture(setup);
      expect(await registry.isEnabled(await operator1.getAddress())).to.be.true;
    });
    it("returns false if the ciphernode is not in the registry", async function () {
      const { registry } = await loadFixture(setup);
      expect(await registry.isCiphernodeEligible(AddressThree)).to.be.false;
    });
  });

  describe("isEnabled()", function () {
    it("returns true if the ciphernode is currently enabled", async function () {
      const { registry, operator1 } = await loadFixture(setup);
      expect(await registry.isEnabled(await operator1.getAddress())).to.be.true;
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
      await registry.requestCommittee(request.e3Id, 0, request.threshold);
      expect(await registry.rootAt(request.e3Id)).to.equal(tree.root);
    });
  });

  describe("treeSize()", function () {
    it("returns the size of the ciphernode registry merkle tree", async function () {
      const { registry, tree } = await loadFixture(setup);
      expect(await registry.treeSize()).to.equal(tree.size);
    });
  });
});
