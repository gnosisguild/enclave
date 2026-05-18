// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import { network } from "hardhat";

import MockCircuitVerifierModule from "../ignition/modules/mockSlashingVerifier";
import {
  BFV_THRESHOLD_T,
  bfvDecExpectedPublicInputsLen,
} from "../scripts/utils";
import {
  BfvDecryptionVerifier__factory as BfvDecryptionVerifierFactory,
  MockCircuitVerifier__factory as MockCircuitVerifierFactory,
} from "../types";

const { ethers, ignition, networkHelpers } = await network.connect();
const { loadFixture } = networkHelpers;

/** Must match `BfvDecryptionVerifier.MESSAGE_COEFFS_COUNT` / circuit `MAX_MSG_NON_ZERO_COEFFS`. */
const MESSAGE_COEFFS_COUNT = 100;

const EXPECTED_C6_FOLD_KEY_HASH = ethers.id("c6_fold");
const EXPECTED_C7_KEY_HASH = ethers.id("c7");

/** Must match `BfvDecryptionVerifier.threshold` / default circuit `T`. */
const THRESHOLD = BFV_THRESHOLD_T;

/** Exact `publicInputs.length` for the configured threshold. */
const EXPECTED_PUBLIC_INPUTS_LEN = bfvDecExpectedPublicInputsLen(THRESHOLD);

function buildPublicInputsWithMessage(
  messageCoeffs: bigint[],
  totalInputs = EXPECTED_PUBLIC_INPUTS_LEN,
  subCircuitHashes: [string, string] = [
    EXPECTED_C6_FOLD_KEY_HASH,
    EXPECTED_C7_KEY_HASH,
  ],
): string[] {
  const arr: string[] = new Array(totalInputs);
  arr[0] = subCircuitHashes[0];
  arr[1] = subCircuitHashes[1];
  for (let i = 2; i < totalInputs; i++) {
    arr[i] = "0x" + "00".repeat(32);
  }
  const offset = totalInputs - MESSAGE_COEFFS_COUNT;
  for (let i = 0; i < messageCoeffs.length && i < MESSAGE_COEFFS_COUNT; i++) {
    arr[offset + i] = "0x" + messageCoeffs[i].toString(16).padStart(64, "0");
  }
  return arr;
}

function plaintextToHash(messageCoeffs: bigint[]): string {
  const buf = new Uint8Array(MESSAGE_COEFFS_COUNT * 8);
  for (
    let i = 0;
    i < Math.min(messageCoeffs.length, MESSAGE_COEFFS_COUNT);
    i++
  ) {
    const c = messageCoeffs[i];
    for (let j = 0; j < 8; j++) {
      buf[i * 8 + j] = Number((c >> BigInt(j * 8)) & 0xffn);
    }
  }
  const hex =
    "0x" +
    Array.from(buf)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
  return ethers.keccak256(hex);
}

function encodeProof(rawProof: string, publicInputs: string[]): string {
  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  return abiCoder.encode(["bytes", "bytes32[]"], [rawProof, publicInputs]);
}

describe("BfvDecryptionVerifier", function () {
  const deployWithMockCircuit = async () => {
    const [owner] = await ethers.getSigners();
    const { mockCircuitVerifier } = await ignition.deploy(
      MockCircuitVerifierModule,
    );
    const mockAddr = await mockCircuitVerifier.getAddress();

    const bfvDecryptionVerifier = await (
      await ethers.getContractFactory("BfvDecryptionVerifier")
    ).deploy(
      mockAddr,
      EXPECTED_C6_FOLD_KEY_HASH,
      EXPECTED_C7_KEY_HASH,
      THRESHOLD,
    );

    await bfvDecryptionVerifier.waitForDeployment();
    const dv = BfvDecryptionVerifierFactory.connect(
      await bfvDecryptionVerifier.getAddress(),
      owner,
    );
    const mc = MockCircuitVerifierFactory.connect(mockAddr, owner);
    return { bfvDecryptionVerifier: dv, mockCircuit: mc };
  };

  describe("reverts / false", function () {
    it("reverts on invalid proof encoding", async function () {
      const { bfvDecryptionVerifier } = await loadFixture(
        deployWithMockCircuit,
      );
      const plaintextHash = ethers.keccak256("0x1234");
      const invalidProof = "0xdeadbeef";

      await expect(
        bfvDecryptionVerifier.verify.staticCall(
          plaintextHash,
          ethers.ZeroHash,
          invalidProof,
        ),
      ).to.be.revert(ethers);
    });

    it("returns false when publicInputs.length is below expected", async function () {
      const { bfvDecryptionVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(true);

      const messageCoeffs = [1n, 2n, 3n];
      const publicInputs = buildPublicInputsWithMessage(
        messageCoeffs,
        EXPECTED_PUBLIC_INPUTS_LEN,
      ).slice(0, EXPECTED_PUBLIC_INPUTS_LEN - 1);
      const plaintextHash = plaintextToHash(messageCoeffs);
      const proof = encodeProof("0x01", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        plaintextHash,
        ethers.ZeroHash,
        proof,
      );
      expect(result).to.equal(false);
    });

    it("returns false when publicInputs.length exceeds expected", async function () {
      const { bfvDecryptionVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(true);

      const messageCoeffs = [1n, 2n, 3n];
      const publicInputs = buildPublicInputsWithMessage(
        messageCoeffs,
        EXPECTED_PUBLIC_INPUTS_LEN + 1,
      );
      const plaintextHash = plaintextToHash(messageCoeffs);
      const proof = encodeProof("0x01", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        plaintextHash,
        ethers.ZeroHash,
        proof,
      );
      expect(result).to.equal(false);
    });

    it("returns false when c6_fold key hash does not match", async function () {
      const revertingVerifier = await (
        await ethers.getContractFactory("RevertOnVerifyCircuitVerifier")
      ).deploy();
      await revertingVerifier.waitForDeployment();

      const bfvDecryptionVerifier = await (
        await ethers.getContractFactory("BfvDecryptionVerifier")
      ).deploy(
        await revertingVerifier.getAddress(),
        EXPECTED_C6_FOLD_KEY_HASH,
        EXPECTED_C7_KEY_HASH,
        THRESHOLD,
      );
      await bfvDecryptionVerifier.waitForDeployment();

      const messageCoeffs = [1n, 2n, 3n];
      const publicInputs = buildPublicInputsWithMessage(
        messageCoeffs,
        EXPECTED_PUBLIC_INPUTS_LEN,
        [ethers.id("wrong-c6"), EXPECTED_C7_KEY_HASH],
      );
      const plaintextHash = plaintextToHash(messageCoeffs);
      const proof = encodeProof("0x01", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        plaintextHash,
        ethers.ZeroHash,
        proof,
      );
      expect(result).to.equal(false);
    });

    it("returns false when c7 key hash does not match", async function () {
      const revertingVerifier = await (
        await ethers.getContractFactory("RevertOnVerifyCircuitVerifier")
      ).deploy();
      await revertingVerifier.waitForDeployment();

      const bfvDecryptionVerifier = await (
        await ethers.getContractFactory("BfvDecryptionVerifier")
      ).deploy(
        await revertingVerifier.getAddress(),
        EXPECTED_C6_FOLD_KEY_HASH,
        EXPECTED_C7_KEY_HASH,
        THRESHOLD,
      );
      await bfvDecryptionVerifier.waitForDeployment();

      const messageCoeffs = [1n, 2n, 3n];
      const publicInputs = buildPublicInputsWithMessage(
        messageCoeffs,
        EXPECTED_PUBLIC_INPUTS_LEN,
        [EXPECTED_C6_FOLD_KEY_HASH, ethers.id("wrong-c7")],
      );
      const plaintextHash = plaintextToHash(messageCoeffs);
      const proof = encodeProof("0x01", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        plaintextHash,
        ethers.ZeroHash,
        proof,
      );
      expect(result).to.equal(false);
    });

    it("returns false when c6_fold key hash does not match", async function () {
      const revertingVerifier = await (
        await ethers.getContractFactory("RevertOnVerifyCircuitVerifier")
      ).deploy();
      await revertingVerifier.waitForDeployment();

      const bfvDecryptionVerifier = await (
        await ethers.getContractFactory("BfvDecryptionVerifier")
      ).deploy(
        await revertingVerifier.getAddress(),
        EXPECTED_C6_FOLD_KEY_HASH,
        EXPECTED_C7_KEY_HASH,
      );
      await bfvDecryptionVerifier.waitForDeployment();

      const messageCoeffs = [1n, 2n, 3n];
      const publicInputs = buildPublicInputsWithMessage(messageCoeffs, 402, [
        ethers.id("wrong-c6"),
        EXPECTED_C7_KEY_HASH,
      ]);
      const plaintextHash = plaintextToHash(messageCoeffs);
      const proof = encodeProof("0x01", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        plaintextHash,
        proof,
      );
      expect(result).to.equal(false);
    });

    it("returns false when c7 key hash does not match", async function () {
      const revertingVerifier = await (
        await ethers.getContractFactory("RevertOnVerifyCircuitVerifier")
      ).deploy();
      await revertingVerifier.waitForDeployment();

      const bfvDecryptionVerifier = await (
        await ethers.getContractFactory("BfvDecryptionVerifier")
      ).deploy(
        await revertingVerifier.getAddress(),
        EXPECTED_C6_FOLD_KEY_HASH,
        EXPECTED_C7_KEY_HASH,
      );
      await bfvDecryptionVerifier.waitForDeployment();

      const messageCoeffs = [1n, 2n, 3n];
      const publicInputs = buildPublicInputsWithMessage(messageCoeffs, 402, [
        EXPECTED_C6_FOLD_KEY_HASH,
        ethers.id("wrong-c7"),
      ]);
      const plaintextHash = plaintextToHash(messageCoeffs);
      const proof = encodeProof("0x01", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        plaintextHash,
        proof,
      );
      expect(result).to.equal(false);
    });

    it("returns false when plaintext hash mismatch", async function () {
      const { bfvDecryptionVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(true);

      const messageCoeffs = [1n, 2n, 3n];
      const publicInputs = buildPublicInputsWithMessage(messageCoeffs);
      const wrongHash = ethers.keccak256("0x0000");
      const proof = encodeProof("0x01", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        wrongHash,
        ethers.ZeroHash,
        proof,
      );
      expect(result).to.equal(false);
    });

    it("returns false when circuit verifier returns false", async function () {
      const { bfvDecryptionVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(false);

      const messageCoeffs = [1n, 2n, 3n];
      const publicInputs = buildPublicInputsWithMessage(messageCoeffs);
      const plaintextHash = plaintextToHash(messageCoeffs);
      const proof = encodeProof("0x01", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        plaintextHash,
        ethers.ZeroHash,
        proof,
      );
      expect(result).to.equal(false);
    });

    it("returns false when constructor expected hashes do not match proof", async function () {
      const { mockCircuit } = await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);
      const mockAddr = await mockCircuit.getAddress();

      const bfvDecryptionVerifier = await (
        await ethers.getContractFactory("BfvDecryptionVerifier")
      ).deploy(
        mockAddr,
        ethers.id("wrong-c6"),
        ethers.id("wrong-c7"),
        THRESHOLD,
      );
      await bfvDecryptionVerifier.waitForDeployment();

      const messageCoeffs = [1n, 2n, 3n];
      const publicInputs = buildPublicInputsWithMessage(messageCoeffs);
      const plaintextHash = plaintextToHash(messageCoeffs);
      const proof = encodeProof("0x0102", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        plaintextHash,
        ethers.ZeroHash,
        proof,
      );
      expect(result).to.equal(false);
    });
  });

  describe("success", function () {
    it("returns true with mock ICircuitVerifier and matching plaintext hash", async function () {
      const { bfvDecryptionVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(true);

      const messageCoeffs = [1n, 2n, 3n, 42n, 100n];
      const publicInputs = buildPublicInputsWithMessage(messageCoeffs);
      const plaintextHash = plaintextToHash(messageCoeffs);
      const proof = encodeProof("0x0102", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        plaintextHash,
        ethers.ZeroHash,
        proof,
      );
      expect(result).to.equal(true);
    });

    it("returns true with exact-length public inputs", async function () {
      const { bfvDecryptionVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(true);

      const messageCoeffs = [1n, 2n, 3n];
      const publicInputs = buildPublicInputsWithMessage(
        messageCoeffs,
        EXPECTED_PUBLIC_INPUTS_LEN,
      );
      const plaintextHash = plaintextToHash(messageCoeffs);
      const proof = encodeProof("0x01", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        plaintextHash,
        ethers.ZeroHash,
        proof,
      );
      expect(result).to.equal(true);
    });
  });
});
