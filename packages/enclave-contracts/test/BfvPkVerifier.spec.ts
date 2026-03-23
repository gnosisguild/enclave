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
    ).deploy(mockAddr);

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
      const invalidProof = "0xdeadbeef";

      await expect(bfvPkVerifier.verify.staticCall(invalidProof)).to.be.revert(
        ethers,
      );
    });

    it("reverts when publicInputs is empty", async function () {
      const { bfvPkVerifier } = await loadFixture(deployWithMockCircuit);
      const proof = encodeProof("0x01", []);

      await expect(bfvPkVerifier.verify.staticCall(proof)).to.be.revert(ethers);
    });

    it("reverts when circuit verifier returns false", async function () {
      const { bfvPkVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(false);

      const commitment = ethers.keccak256("0x1234");
      const proof = encodeProof("0x01", [commitment]);

      await expect(bfvPkVerifier.verify.staticCall(proof)).to.be.revert(ethers);
    });
  });

  describe("success", function () {
    it("returns publicInputs[last] when circuit verifier returns true", async function () {
      const { bfvPkVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(true);

      const commitment = ethers.keccak256("0xabcd");
      const proof = encodeProof("0x0102", [
        "0x" + "00".repeat(32),
        "0x" + "00".repeat(32),
        commitment,
      ]);

      const result = await bfvPkVerifier.verify.staticCall(proof);
      expect(result).to.equal(commitment);
    });

    it("returns commitment with single public input", async function () {
      const { bfvPkVerifier, mockCircuit } = await loadFixture(
        deployWithMockCircuit,
      );
      await mockCircuit.setReturnValue(true);

      const commitment = ethers.id("committee-pk");
      const proof = encodeProof("0x", [commitment]);

      const result = await bfvPkVerifier.verify.staticCall(proof);
      expect(result).to.equal(commitment);
    });
  });
});
