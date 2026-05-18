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

const EXPECTED_NODES_FOLD_KEY_HASH = ethers.id("nodes_fold");
const EXPECTED_C5_KEY_HASH = ethers.id("c5");

function encodeProof(rawProof: string, publicInputs: string[]): string {
  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  return abiCoder.encode(["bytes", "bytes32[]"], [rawProof, publicInputs]);
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
    ).deploy(mockAddr, EXPECTED_NODES_FOLD_KEY_HASH, EXPECTED_C5_KEY_HASH);

    await bfvPkVerifier.waitForDeployment();
    const pk = BfvPkVerifierFactory.connect(
      await bfvPkVerifier.getAddress(),
      owner,
    );
    const mc = MockCircuitVerifierFactory.connect(mockAddr, owner);
    return { bfvPkVerifier: pk, mockCircuit: mc };
  };

  describe("reverts / false", function () {
    it("reverts on invalid proof encoding", async function () {
      const { bfvPkVerifier } = await loadFixture(deployWithMockCircuit);
      const pkCommitment = ethers.keccak256("0x1234");
      const invalidProof = "0xdeadbeef";

      await expect(
        bfvPkVerifier.verify.staticCall(pkCommitment, invalidProof),
      ).to.be.revert(ethers);
    });

    it("returns false when publicInputs is empty", async function () {
      const { bfvPkVerifier } = await loadFixture(deployWithMockCircuit);
      const pkCommitment = ethers.keccak256("0x1234");
      const proof = encodeProof("0x01", []);

      const result = await bfvPkVerifier.verify.staticCall(pkCommitment, proof);
      expect(result).to.equal(false);
    });

    it("returns false when publicInputs has only one entry", async function () {
      const { bfvPkVerifier } = await loadFixture(deployWithMockCircuit);
      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x01", [pkCommitment]);

      const result = await bfvPkVerifier.verify.staticCall(pkCommitment, proof);
      expect(result).to.equal(false);
    });

    it("returns false when publicInputs has only vk hashes (no pkCommitment slot)", async function () {
      const { bfvPkVerifier } = await loadFixture(deployWithMockCircuit);
      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x01", [
        EXPECTED_NODES_FOLD_KEY_HASH,
        EXPECTED_C5_KEY_HASH,
      ]);

      const result = await bfvPkVerifier.verify.staticCall(pkCommitment, proof);
      expect(result).to.equal(false);
    });

    it("returns false when nodes_fold key hash does not match", async function () {
      const revertingVerifier = await (
        await ethers.getContractFactory("RevertOnVerifyCircuitVerifier")
      ).deploy();
      await revertingVerifier.waitForDeployment();

      const bfvPkVerifier = await (
        await ethers.getContractFactory("BfvPkVerifier")
      ).deploy(
        await revertingVerifier.getAddress(),
        EXPECTED_NODES_FOLD_KEY_HASH,
        EXPECTED_C5_KEY_HASH,
      );
      await bfvPkVerifier.waitForDeployment();

      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x01", [
        ethers.id("wrong-nodes-fold"),
        EXPECTED_C5_KEY_HASH,
        pkCommitment,
      ]);

      const result = await bfvPkVerifier.verify.staticCall(pkCommitment, proof);
      expect(result).to.equal(false);
    });

    it("returns false when c5 key hash does not match", async function () {
      const revertingVerifier = await (
        await ethers.getContractFactory("RevertOnVerifyCircuitVerifier")
      ).deploy();
      await revertingVerifier.waitForDeployment();

      const bfvPkVerifier = await (
        await ethers.getContractFactory("BfvPkVerifier")
      ).deploy(
        await revertingVerifier.getAddress(),
        EXPECTED_NODES_FOLD_KEY_HASH,
        EXPECTED_C5_KEY_HASH,
      );
      await bfvPkVerifier.waitForDeployment();

      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x01", [
        EXPECTED_NODES_FOLD_KEY_HASH,
        ethers.id("wrong-c5"),
        pkCommitment,
      ]);

      const result = await bfvPkVerifier.verify.staticCall(pkCommitment, proof);
      expect(result).to.equal(false);
    });

    it("returns false when pkCommitment does not match last public input", async function () {
      const { bfvPkVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(true);

      const pkCommitment = ethers.keccak256("0xabcd");
      const wrong = ethers.keccak256("0x1234");
      const proof = encodeProof("0x01", [
        EXPECTED_NODES_FOLD_KEY_HASH,
        EXPECTED_C5_KEY_HASH,
        wrong,
      ]);

      const result = await bfvPkVerifier.verify.staticCall(pkCommitment, proof);
      expect(result).to.equal(false);
    });

    it("returns false when circuit verifier returns false", async function () {
      const { bfvPkVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(false);

      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x01", [
        EXPECTED_NODES_FOLD_KEY_HASH,
        EXPECTED_C5_KEY_HASH,
        pkCommitment,
      ]);

      const result = await bfvPkVerifier.verify.staticCall(pkCommitment, proof);
      expect(result).to.equal(false);
    });

    it("returns false when constructor expected hashes do not match proof", async function () {
      const { mockCircuit } = await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);
      const mockAddr = await mockCircuit.getAddress();

      const bfvPkVerifier = await (
        await ethers.getContractFactory("BfvPkVerifier")
      ).deploy(mockAddr, ethers.id("wrong-nodes-fold"), ethers.id("wrong-c5"));
      await bfvPkVerifier.waitForDeployment();

      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x0102", [
        EXPECTED_NODES_FOLD_KEY_HASH,
        EXPECTED_C5_KEY_HASH,
        pkCommitment,
      ]);

      const result = await bfvPkVerifier.verify.staticCall(pkCommitment, proof);
      expect(result).to.equal(false);
    });
  });

  describe("success", function () {
    it("returns true when commitment matches and circuit verifier passes", async function () {
      const { bfvPkVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(true);

      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x0102", [
        EXPECTED_NODES_FOLD_KEY_HASH,
        EXPECTED_C5_KEY_HASH,
        "0x" + "00".repeat(32),
        pkCommitment,
      ]);

      const result = await bfvPkVerifier.verify.staticCall(pkCommitment, proof);
      expect(result).to.equal(true);
    });
  });
});
