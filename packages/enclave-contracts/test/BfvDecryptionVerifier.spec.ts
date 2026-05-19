// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import { network } from "hardhat";

import MockCircuitVerifierModule from "../ignition/modules/mockSlashingVerifier";
import {
  BfvDecryptionVerifier__factory as BfvDecryptionVerifierFactory,
  MockCircuitVerifier__factory as MockCircuitVerifierFactory,
} from "../types";

const { ethers, ignition, networkHelpers } = await network.connect();
const { loadFixture } = networkHelpers;

/** Must match `BfvDecryptionVerifier.MESSAGE_COEFFS_COUNT`. */
const MESSAGE_COEFFS_COUNT = 100;
const C6_FOLD_KEY_HASH = ethers.id("test:c6-fold-vk");
const C7_KEY_HASH = ethers.id("test:c7-vk");

function buildPublicInputs(
  domainBinding: string,
  messageCoeffs: bigint[],
  totalInputs = 402,
): string[] {
  const arr: string[] = new Array(totalInputs);
  for (let i = 0; i < totalInputs; i++) arr[i] = "0x" + "00".repeat(32);
  arr[0] = C6_FOLD_KEY_HASH;
  arr[1] = C7_KEY_HASH;
  arr[totalInputs - MESSAGE_COEFFS_COUNT - 1] = domainBinding;
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

function computeDomainBinding(
  verifierAddr: string,
  chainId: bigint,
  e3Id: bigint,
  committeeRoot: bigint,
  sortedNodes: string[],
  ciphertextOutputHash: string,
  committeePublicKey: string,
  plaintextOutputHash: string,
): string {
  return ethers.keccak256(
    ethers.AbiCoder.defaultAbiCoder().encode(
      [
        "uint256",
        "address",
        "uint256",
        "uint256",
        "address[]",
        "bytes32",
        "bytes32",
        "bytes32",
      ],
      [
        chainId,
        verifierAddr,
        e3Id,
        committeeRoot,
        sortedNodes,
        ciphertextOutputHash,
        committeePublicKey,
        plaintextOutputHash,
      ],
    ),
  );
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
    ).deploy(mockAddr, C6_FOLD_KEY_HASH, C7_KEY_HASH);

    await bfvDecryptionVerifier.waitForDeployment();
    const dv = BfvDecryptionVerifierFactory.connect(
      await bfvDecryptionVerifier.getAddress(),
      owner,
    );
    const mc = MockCircuitVerifierFactory.connect(mockAddr, owner);
    const chainId = (await ethers.provider.getNetwork()).chainId;
    return {
      bfvDecryptionVerifier: dv,
      mockCircuit: mc,
      verifierAddr: await dv.getAddress(),
      chainId,
    };
  };

  const ctx = (verifierAddr: string) => {
    const e3Id = 7n;
    const root = 1234n;
    const nodes = [verifierAddr, ethers.ZeroAddress];
    const ciphertextHash = ethers.id("ct-hash");
    const committeePk = ethers.id("committee-pk");
    return { e3Id, root, nodes, ciphertextHash, committeePk };
  };

  describe("reverts", function () {
    it("reverts on invalid proof encoding", async function () {
      const { bfvDecryptionVerifier, verifierAddr } = await loadFixture(
        deployWithMockCircuit,
      );
      const { e3Id, root, nodes, ciphertextHash, committeePk } =
        ctx(verifierAddr);
      const plaintextHash = ethers.keccak256("0x1234");
      await expect(
        bfvDecryptionVerifier.verify.staticCall(
          e3Id,
          root,
          nodes,
          ciphertextHash,
          committeePk,
          plaintextHash,
          "0xdeadbeef",
        ),
      ).to.be.revert(ethers);
    });

    it("reverts InvalidPublicInputsLength when too short", async function () {
      const { bfvDecryptionVerifier, verifierAddr } = await loadFixture(
        deployWithMockCircuit,
      );
      const { e3Id, root, nodes, ciphertextHash, committeePk } =
        ctx(verifierAddr);
      const plaintextHash = ethers.keccak256("0x1234");
      // need >= MESSAGE_COEFFS_COUNT + 3 = 103
      const publicInputs = new Array(102).fill("0x" + "00".repeat(32));
      const proof = encodeProof("0x01", publicInputs);

      await expect(
        bfvDecryptionVerifier.verify.staticCall(
          e3Id,
          root,
          nodes,
          ciphertextHash,
          committeePk,
          plaintextHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(
        bfvDecryptionVerifier,
        "InvalidPublicInputsLength",
      );
    });

    it("reverts VkHashMismatch when slot 0 differs (M-34)", async function () {
      const { bfvDecryptionVerifier, mockCircuit, verifierAddr, chainId } =
        await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);
      const { e3Id, root, nodes, ciphertextHash, committeePk } =
        ctx(verifierAddr);

      const messageCoeffs = [1n, 2n, 3n];
      const plaintextHash = plaintextToHash(messageCoeffs);
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        e3Id,
        root,
        nodes,
        ciphertextHash,
        committeePk,
        plaintextHash,
      );
      const publicInputs = buildPublicInputs(binding, messageCoeffs);
      publicInputs[0] = ethers.id("wrong-vk");
      const proof = encodeProof("0x01", publicInputs);

      await expect(
        bfvDecryptionVerifier.verify.staticCall(
          e3Id,
          root,
          nodes,
          ciphertextHash,
          committeePk,
          plaintextHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(bfvDecryptionVerifier, "VkHashMismatch");
    });

    it("reverts VkHashMismatch when slot 1 differs (M-34)", async function () {
      const { bfvDecryptionVerifier, mockCircuit, verifierAddr, chainId } =
        await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);
      const { e3Id, root, nodes, ciphertextHash, committeePk } =
        ctx(verifierAddr);

      const messageCoeffs = [1n, 2n, 3n];
      const plaintextHash = plaintextToHash(messageCoeffs);
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        e3Id,
        root,
        nodes,
        ciphertextHash,
        committeePk,
        plaintextHash,
      );
      const publicInputs = buildPublicInputs(binding, messageCoeffs);
      publicInputs[1] = ethers.id("wrong-c7-vk");
      const proof = encodeProof("0x01", publicInputs);

      await expect(
        bfvDecryptionVerifier.verify.staticCall(
          e3Id,
          root,
          nodes,
          ciphertextHash,
          committeePk,
          plaintextHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(bfvDecryptionVerifier, "VkHashMismatch");
    });

    it("reverts PlaintextHashMismatch when message coeffs don't hash to plaintextHash", async function () {
      const { bfvDecryptionVerifier, mockCircuit, verifierAddr, chainId } =
        await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);
      const { e3Id, root, nodes, ciphertextHash, committeePk } =
        ctx(verifierAddr);

      const messageCoeffs = [1n, 2n, 3n];
      const realHash = plaintextToHash(messageCoeffs);
      const wrongHash = ethers.keccak256("0x0000");
      // build a valid binding for the wrong hash (so binding check passes)
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        e3Id,
        root,
        nodes,
        ciphertextHash,
        committeePk,
        wrongHash,
      );
      const publicInputs = buildPublicInputs(binding, messageCoeffs);
      const proof = encodeProof("0x01", publicInputs);
      // sanity: coeffs hash != wrongHash
      expect(realHash).to.not.equal(wrongHash);

      await expect(
        bfvDecryptionVerifier.verify.staticCall(
          e3Id,
          root,
          nodes,
          ciphertextHash,
          committeePk,
          wrongHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(
        bfvDecryptionVerifier,
        "PlaintextHashMismatch",
      );
    });

    it("reverts DomainBindingMismatch on replay across e3Id (C-08)", async function () {
      const { bfvDecryptionVerifier, mockCircuit, verifierAddr, chainId } =
        await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);
      const { root, nodes, ciphertextHash, committeePk } = ctx(verifierAddr);

      const messageCoeffs = [1n, 2n, 3n];
      const plaintextHash = plaintextToHash(messageCoeffs);
      // build proof for e3Id=1
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        1n,
        root,
        nodes,
        ciphertextHash,
        committeePk,
        plaintextHash,
      );
      const publicInputs = buildPublicInputs(binding, messageCoeffs);
      const proof = encodeProof("0x01", publicInputs);

      // verify for e3Id=2 -> mismatch
      await expect(
        bfvDecryptionVerifier.verify.staticCall(
          2,
          root,
          nodes,
          ciphertextHash,
          committeePk,
          plaintextHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(
        bfvDecryptionVerifier,
        "DomainBindingMismatch",
      );
    });

    it("reverts DomainBindingMismatch on replay across wrapper address (C-08)", async function () {
      const {
        mockCircuit,
        verifierAddr: addr1,
        chainId,
      } = await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);
      const mockAddr = await mockCircuit.getAddress();

      const dv2 = await (
        await ethers.getContractFactory("BfvDecryptionVerifier")
      ).deploy(mockAddr, C6_FOLD_KEY_HASH, C7_KEY_HASH);
      await dv2.waitForDeployment();
      const addr2 = await dv2.getAddress();
      expect(addr2).to.not.equal(addr1);

      const e3Id = 1n;
      const root = 0n;
      const nodes = [addr1];
      const ciphertextHash = ethers.id("ct");
      const committeePk = ethers.id("pk");
      const messageCoeffs = [1n, 2n];
      const plaintextHash = plaintextToHash(messageCoeffs);
      // binding for wrapper #1
      const binding = computeDomainBinding(
        addr1,
        chainId,
        e3Id,
        root,
        nodes,
        ciphertextHash,
        committeePk,
        plaintextHash,
      );
      const publicInputs = buildPublicInputs(binding, messageCoeffs);
      const proof = encodeProof("0x01", publicInputs);

      // call wrapper #2 -> binding for #1 won't match
      await expect(
        dv2.verify.staticCall(
          e3Id,
          root,
          nodes,
          ciphertextHash,
          committeePk,
          plaintextHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(dv2, "DomainBindingMismatch");
    });

    it("reverts InvalidProof when underlying honk verifier rejects (M-35)", async function () {
      const { bfvDecryptionVerifier, mockCircuit, verifierAddr, chainId } =
        await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(false);
      const { e3Id, root, nodes, ciphertextHash, committeePk } =
        ctx(verifierAddr);

      const messageCoeffs = [1n, 2n, 3n];
      const plaintextHash = plaintextToHash(messageCoeffs);
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        e3Id,
        root,
        nodes,
        ciphertextHash,
        committeePk,
        plaintextHash,
      );
      const publicInputs = buildPublicInputs(binding, messageCoeffs);
      const proof = encodeProof("0x01", publicInputs);

      await expect(
        bfvDecryptionVerifier.verify.staticCall(
          e3Id,
          root,
          nodes,
          ciphertextHash,
          committeePk,
          plaintextHash,
          proof,
        ),
      ).to.be.revertedWithCustomError(bfvDecryptionVerifier, "InvalidProof");
    });
  });

  describe("success", function () {
    it("returns true when all checks pass", async function () {
      const { bfvDecryptionVerifier, mockCircuit, verifierAddr, chainId } =
        await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);
      const { e3Id, root, nodes, ciphertextHash, committeePk } =
        ctx(verifierAddr);

      const messageCoeffs = [1n, 2n, 3n, 42n, 100n];
      const plaintextHash = plaintextToHash(messageCoeffs);
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        e3Id,
        root,
        nodes,
        ciphertextHash,
        committeePk,
        plaintextHash,
      );
      const publicInputs = buildPublicInputs(binding, messageCoeffs);
      const proof = encodeProof("0x0102", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        e3Id,
        root,
        nodes,
        ciphertextHash,
        committeePk,
        plaintextHash,
        proof,
      );
      expect(result).to.equal(true);
    });

    it("returns true with minimal public inputs (totalInputs == 103)", async function () {
      const { bfvDecryptionVerifier, mockCircuit, verifierAddr, chainId } =
        await loadFixture(deployWithMockCircuit);
      await mockCircuit.setReturnValue(true);
      const { e3Id, root, nodes, ciphertextHash, committeePk } =
        ctx(verifierAddr);

      const messageCoeffs = [1n, 2n, 3n];
      const plaintextHash = plaintextToHash(messageCoeffs);
      const binding = computeDomainBinding(
        verifierAddr,
        chainId,
        e3Id,
        root,
        nodes,
        ciphertextHash,
        committeePk,
        plaintextHash,
      );
      const publicInputs = buildPublicInputs(
        binding,
        messageCoeffs,
        MESSAGE_COEFFS_COUNT + 3,
      );
      const proof = encodeProof("0x01", publicInputs);

      const result = await bfvDecryptionVerifier.verify.staticCall(
        e3Id,
        root,
        nodes,
        ciphertextHash,
        committeePk,
        plaintextHash,
        proof,
      );
      expect(result).to.equal(true);
    });
  });
});
