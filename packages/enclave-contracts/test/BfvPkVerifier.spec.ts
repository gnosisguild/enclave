// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import { network } from "hardhat";

import MockCircuitVerifierModule from "../ignition/modules/mockSlashingVerifier";
import {
  BfvPkVerifier__factory as BfvPkVerifierFactory,
  MockCircuitVerifier__factory as MockCircuitVerifierFactory,
} from "../types";

const { ethers, ignition, networkHelpers } = await network.connect();
const { loadFixture } = networkHelpers;

const NODES_FOLD_KEY_HASH = ethers.id("test:nodes-fold-vk");
const C5_KEY_HASH = ethers.id("test:c5-vk");

function encodeProof(rawProof: string, publicInputs: string[]): string {
  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  return abiCoder.encode(["bytes", "bytes32[]"], [rawProof, publicInputs]);
}

function computeDomainBinding(
  verifierAddr: string,
  chainId: bigint,
  e3Id: bigint,
  committeeRoot: bigint,
  sortedNodes: string[],
  pkCommitment: string,
): string {
  return ethers.keccak256(
    ethers.AbiCoder.defaultAbiCoder().encode(
      ["uint256", "address", "uint256", "uint256", "address[]", "bytes32"],
      [chainId, verifierAddr, e3Id, committeeRoot, sortedNodes, pkCommitment],
    ),
  );
}

function buildValidPublicInputs(
  domainBinding: string,
  pkCommitment: string,
): string[] {
  return [
    NODES_FOLD_KEY_HASH,
    C5_KEY_HASH,
    "0x" + "11".repeat(32),
    domainBinding,
    pkCommitment,
  ];
}

describe("BfvPkVerifier", function () {
  const deployWithMockCircuit = async () => {
    const [owner] = await ethers.getSigners();
    const { mockCircuitVerifier } = await ignition.deploy(
      MockCircuitVerifierModule,
    );
    const mockAddr = await mockCircuitVerifier.getAddress();

    const bfvPkVerifier = await (
      await ethers.getContractFactory("BfvPkVerifier")
    ).deploy(mockAddr, NODES_FOLD_KEY_HASH, C5_KEY_HASH);

    await bfvPkVerifier.waitForDeployment();
    const pk = BfvPkVerifierFactory.connect(
      await bfvPkVerifier.getAddress(),
      owner,
    );
    const mc = MockCircuitVerifierFactory.connect(mockAddr, owner);
    const chainId = (await ethers.provider.getNetwork()).chainId;
    return {
      bfvPkVerifier: pk,
      mockCircuit: mc,
      verifierAddr: await pk.getAddress(),
      chainId,
    };
  };

  describe("reverts", function () {
    it("reverts on invalid proof encoding", async function () {
      const { bfvPkVerifier, verifierAddr } = await loadFixture(
        deployWithMockCircuit,
      );
      const pkCommitment = ethers.keccak256("0x1234");
      const invalidProof = "0xdeadbeef";

      await expect(
        bfvPkVerifier.verify.staticCall(
          1,
          0,
          [verifierAddr],
          pkCommitment,
          invalidProof,
        ),
      ).to.be.revert(ethers);
    });

    it("reverts InvalidPublicInputsLength when publicInputs is empty", async function () {
      const { bfvPkVerifier, verifierAddr } = await loadFixture(
        deployWithMockCircuit,
      );
      const pkCommitment = ethers.keccak256("0x1234");
      const proof = encodeProof("0x01", []);

      await expect(
        bfvPkVerifier.verify.staticCall(
          1,
          0,
          [verifierAddr],
          pkCommitment,
          proof,
        ),
      ).to.be.revertedWithCustomError(
        bfvPkVerifier,
        "InvalidPublicInputsLength",
      );
    });

    it("reverts VkHashMismatch when slot 0 differs (M-34)", async function () {
      const { bfvPkVerifier, verifierAddr, chainId } = await loadFixture(
        deployWithMockCircuit,
      );
      const pkCommitment = ethers.keccak256("0xabcd");
      const e3Id = 7n;
      const root = 0n;
      const nodes = [verifierAddr];
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        e3Id,
        root,
        nodes,
        pkCommitment,
      );
      const publicInputs = buildValidPublicInputs(binding, pkCommitment);
      publicInputs[0] = ethers.id("wrong-vk");
      const proof = encodeProof("0x01", publicInputs);

      await expect(
        bfvPkVerifier.verify.staticCall(e3Id, root, nodes, pkCommitment, proof),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "VkHashMismatch");
    });

    it("reverts VkHashMismatch when slot 1 differs (M-34)", async function () {
      const { bfvPkVerifier, verifierAddr, chainId } = await loadFixture(
        deployWithMockCircuit,
      );
      const pkCommitment = ethers.keccak256("0xabcd");
      const e3Id = 7n;
      const root = 0n;
      const nodes = [verifierAddr];
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        e3Id,
        root,
        nodes,
        pkCommitment,
      );
      const publicInputs = buildValidPublicInputs(binding, pkCommitment);
      publicInputs[1] = ethers.id("wrong-c5-vk");
      const proof = encodeProof("0x01", publicInputs);

      await expect(
        bfvPkVerifier.verify.staticCall(e3Id, root, nodes, pkCommitment, proof),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "VkHashMismatch");
    });

    it("reverts PkCommitmentMismatch when last slot != pkCommitment", async function () {
      const { bfvPkVerifier, verifierAddr, chainId } = await loadFixture(
        deployWithMockCircuit,
      );
      const pkCommitment = ethers.keccak256("0xabcd");
      const wrongCommitment = ethers.keccak256("0xbeef");
      const e3Id = 7n;
      const root = 0n;
      const nodes = [verifierAddr];
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        e3Id,
        root,
        nodes,
        pkCommitment,
      );
      const publicInputs = buildValidPublicInputs(binding, wrongCommitment);
      const proof = encodeProof("0x01", publicInputs);

      await expect(
        bfvPkVerifier.verify.staticCall(e3Id, root, nodes, pkCommitment, proof),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "PkCommitmentMismatch");
    });

    it("reverts DomainBindingMismatch on replay across e3Id (C-08)", async function () {
      const { bfvPkVerifier, mockCircuit, verifierAddr, chainId } =
        await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);

      const pkCommitment = ethers.keccak256("0xabcd");
      const root = 0n;
      const nodes = [verifierAddr];
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        1n,
        root,
        nodes,
        pkCommitment,
      );
      const publicInputs = buildValidPublicInputs(binding, pkCommitment);
      const proof = encodeProof("0x01", publicInputs);

      await expect(
        bfvPkVerifier.verify.staticCall(2, root, nodes, pkCommitment, proof),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "DomainBindingMismatch");
    });

    it("reverts DomainBindingMismatch on replay across wrapper address (C-08)", async function () {
      const {
        mockCircuit,
        verifierAddr: addr1,
        chainId,
      } = await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);
      const mockAddr = await mockCircuit.getAddress();

      const bfv2 = await (
        await ethers.getContractFactory("BfvPkVerifier")
      ).deploy(mockAddr, NODES_FOLD_KEY_HASH, C5_KEY_HASH);
      await bfv2.waitForDeployment();
      const addr2 = await bfv2.getAddress();
      expect(addr2).to.not.equal(addr1);

      const pkCommitment = ethers.keccak256("0xabcd");
      const e3Id = 1n;
      const root = 0n;
      const nodes = [addr1];
      const binding = computeDomainBinding(
        addr1,
        chainId,
        e3Id,
        root,
        nodes,
        pkCommitment,
      );
      const publicInputs = buildValidPublicInputs(binding, pkCommitment);
      const proof = encodeProof("0x01", publicInputs);

      await expect(
        bfv2.verify.staticCall(e3Id, root, nodes, pkCommitment, proof),
      ).to.be.revertedWithCustomError(bfv2, "DomainBindingMismatch");
    });

    it("reverts InvalidProof when underlying honk verifier rejects (M-35)", async function () {
      const { bfvPkVerifier, mockCircuit, verifierAddr, chainId } =
        await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(false);

      const pkCommitment = ethers.keccak256("0xabcd");
      const e3Id = 1n;
      const root = 0n;
      const nodes = [verifierAddr];
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        e3Id,
        root,
        nodes,
        pkCommitment,
      );
      const publicInputs = buildValidPublicInputs(binding, pkCommitment);
      const proof = encodeProof("0x01", publicInputs);

      await expect(
        bfvPkVerifier.verify.staticCall(e3Id, root, nodes, pkCommitment, proof),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "InvalidProof");
    });
  });

  describe("success", function () {
    it("returns true when all checks pass", async function () {
      const { bfvPkVerifier, mockCircuit, verifierAddr, chainId } =
        await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);

      const pkCommitment = ethers.id("committee-pk");
      const e3Id = 42n;
      const root = 0xdeadbeefn;
      const nodes = [verifierAddr, ethers.ZeroAddress];
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        e3Id,
        root,
        nodes,
        pkCommitment,
      );
      const publicInputs = buildValidPublicInputs(binding, pkCommitment);
      const proof = encodeProof("0x0102", publicInputs);

      const result = await bfvPkVerifier.verify.staticCall(
        e3Id,
        root,
        nodes,
        pkCommitment,
        proof,
      );
      expect(result).to.equal(true);
    });
  });
});
