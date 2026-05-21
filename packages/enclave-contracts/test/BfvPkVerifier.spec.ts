// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import { network } from "hardhat";

import MockCircuitVerifierModule from "../ignition/modules/mockSlashingVerifier";
import { BFV_DKG_H } from "../scripts/utils";
import {
  BfvPkVerifier__factory as BfvPkVerifierFactory,
  MockCircuitVerifier__factory as MockCircuitVerifierFactory,
} from "../types";

const { ethers, ignition, networkHelpers } = await network.connect();
const { loadFixture } = networkHelpers;

const EXPECTED_NODES_FOLD_KEY_HASH = ethers.id("nodes_fold");
const EXPECTED_C5_KEY_HASH = ethers.id("c5");
/** Must match `BfvPkVerifier.h` / default circuit `H`. */
const H = BFV_DKG_H;
const DKG_RETURN_FIELD_COUNT = 8;

function committeeHashLimbs(committeeHash: string): [string, string] {
  const bn = BigInt(committeeHash);
  const hi = ethers.toBeHex(bn >> 128n, 32);
  const lo = ethers.toBeHex(bn & ((1n << 128n) - 1n), 32);
  return [hi, lo];
}

function minimalDkgPublicInputs(
  pkCommitment: string,
  committeeHash: string = ethers.ZeroHash,
): string[] {
  const [hi, lo] = committeeHashLimbs(committeeHash);
  return [
    EXPECTED_NODES_FOLD_KEY_HASH,
    EXPECTED_C5_KEY_HASH,
    ...Array(H).fill(ethers.ZeroHash),
    hi,
    lo,
    ...Array(DKG_RETURN_FIELD_COUNT - 1).fill(ethers.ZeroHash),
    pkCommitment,
  ];
}

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
    ).deploy(mockAddr, EXPECTED_NODES_FOLD_KEY_HASH, EXPECTED_C5_KEY_HASH, H);

    await bfvPkVerifier.waitForDeployment();
    const pk = BfvPkVerifierFactory.connect(
      await bfvPkVerifier.getAddress(),
      owner,
    );
    const mc = MockCircuitVerifierFactory.connect(mockAddr, owner);
    return { bfvPkVerifier: pk, mockCircuit: mc };
  };

  describe("reverts", function () {
    it("reverts on invalid proof encoding", async function () {
      const { bfvPkVerifier } = await loadFixture(deployWithMockCircuit);
      const pkCommitment = ethers.keccak256("0x1234");
      const invalidProof = "0xdeadbeef";

      await expect(
        bfvPkVerifier.verify.staticCall(
          pkCommitment,
          ethers.ZeroHash,
          invalidProof,
        ),
      ).to.be.revert(ethers);
    });

    it("reverts with BadPublicInputsLen when publicInputs is empty", async function () {
      const { bfvPkVerifier } = await loadFixture(deployWithMockCircuit);
      const pkCommitment = ethers.keccak256("0x1234");
      const proof = encodeProof("0x01", []);

      await expect(
        bfvPkVerifier.verify.staticCall(
          pkCommitment,
          ethers.ZeroHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "BadPublicInputsLen");
    });

    it("reverts with BadPublicInputsLen when publicInputs has only one entry", async function () {
      const { bfvPkVerifier } = await loadFixture(deployWithMockCircuit);
      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x01", [pkCommitment]);

      await expect(
        bfvPkVerifier.verify.staticCall(
          pkCommitment,
          ethers.ZeroHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "BadPublicInputsLen");
    });

    it("reverts with BadPublicInputsLen when publicInputs has trailing elements past expected length", async function () {
      const { bfvPkVerifier } = await loadFixture(deployWithMockCircuit);
      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x01", [
        ...minimalDkgPublicInputs(pkCommitment),
        ethers.ZeroHash,
      ]);

      await expect(
        bfvPkVerifier.verify.staticCall(
          pkCommitment,
          ethers.ZeroHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "BadPublicInputsLen");
    });

    it("reverts with BadPublicInputsLen when publicInputs has only pub params (no return fields)", async function () {
      const { bfvPkVerifier } = await loadFixture(deployWithMockCircuit);
      const pkCommitment = ethers.keccak256("0xabcd");
      const [hi, lo] = committeeHashLimbs(ethers.ZeroHash);
      const proof = encodeProof("0x01", [
        EXPECTED_NODES_FOLD_KEY_HASH,
        EXPECTED_C5_KEY_HASH,
        ...Array(H).fill(ethers.ZeroHash),
        hi,
        lo,
        pkCommitment,
      ]);

      await expect(
        bfvPkVerifier.verify.staticCall(
          pkCommitment,
          ethers.ZeroHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "BadPublicInputsLen");
    });

    it("reverts with BadPublicInputsLen when publicInputs has only vk hashes (no pkCommitment slot)", async function () {
      const { bfvPkVerifier } = await loadFixture(deployWithMockCircuit);
      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x01", [
        EXPECTED_NODES_FOLD_KEY_HASH,
        EXPECTED_C5_KEY_HASH,
      ]);

      await expect(
        bfvPkVerifier.verify.staticCall(
          pkCommitment,
          ethers.ZeroHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "BadPublicInputsLen");
    });

    it("reverts with BadNodesFoldKeyHash when nodes_fold key hash does not match", async function () {
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
        H,
      );
      await bfvPkVerifier.waitForDeployment();

      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof(
        "0x01",
        minimalDkgPublicInputs(pkCommitment).map((v, i) =>
          i === 0 ? ethers.id("wrong-nodes-fold") : v,
        ),
      );

      await expect(
        bfvPkVerifier.verify.staticCall(
          pkCommitment,
          ethers.ZeroHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "BadNodesFoldKeyHash");
    });

    it("reverts with BadC5KeyHash when c5 key hash does not match", async function () {
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
        H,
      );
      await bfvPkVerifier.waitForDeployment();

      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof(
        "0x01",
        minimalDkgPublicInputs(pkCommitment).map((v, i) =>
          i === 1 ? ethers.id("wrong-c5") : v,
        ),
      );

      await expect(
        bfvPkVerifier.verify.staticCall(
          pkCommitment,
          ethers.ZeroHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "BadC5KeyHash");
    });

    it("reverts with BadPkCommitment when pkCommitment does not match last public input", async function () {
      const { bfvPkVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(true);

      const pkCommitment = ethers.keccak256("0xabcd");
      const wrong = ethers.keccak256("0x1234");
      const proof = encodeProof("0x01", minimalDkgPublicInputs(wrong));

      await expect(
        bfvPkVerifier.verify.staticCall(
          pkCommitment,
          ethers.ZeroHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "BadPkCommitment");
    });

    it("returns false when circuit verifier returns false", async function () {
      // Circuit-verify failure is still expressed as `return false` because the
      // bool return reflects the underlying Honk verifier's bool result.
      const { bfvPkVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(false);

      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x01", minimalDkgPublicInputs(pkCommitment));

      const result = await bfvPkVerifier.verify.staticCall(
        pkCommitment,
        ethers.ZeroHash,
        proof,
      );
      expect(result).to.equal(false);
    });

    it("reverts with BadNodesFoldKeyHash when constructor expected hashes do not match proof", async function () {
      const { mockCircuit } = await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);
      const mockAddr = await mockCircuit.getAddress();

      const bfvPkVerifier = await (
        await ethers.getContractFactory("BfvPkVerifier")
      ).deploy(
        mockAddr,
        ethers.id("wrong-nodes-fold"),
        ethers.id("wrong-c5"),
        H,
      );
      await bfvPkVerifier.waitForDeployment();

      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x0102", minimalDkgPublicInputs(pkCommitment));

      // The proof's nodes_fold hash is checked first and won't match the
      // constructor's "wrong-nodes-fold", so BadNodesFoldKeyHash fires.
      await expect(
        bfvPkVerifier.verify.staticCall(
          pkCommitment,
          ethers.ZeroHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(bfvPkVerifier, "BadNodesFoldKeyHash");
    });
  });

  describe("success", function () {
    it("returns true when commitment matches and circuit verifier passes", async function () {
      const { bfvPkVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(true);

      const pkCommitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x0102", minimalDkgPublicInputs(pkCommitment));

      const result = await bfvPkVerifier.verify.staticCall(
        pkCommitment,
        ethers.ZeroHash,
        proof,
      );
      expect(result).to.equal(true);
    });
  });
});
