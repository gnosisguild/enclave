// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import { network } from "hardhat";
import fs from "node:fs";
import path from "node:path";

import {
  BFV_DECRYPTION_SUB_CIRCUIT_VK_HASH_PATHS,
  BFV_PK_SUB_CIRCUIT_VK_HASH_PATHS,
  REPO_ROOT,
  assertBfvDecryptionVerifierSubCircuitVkHashes,
  assertBfvPkVerifierSubCircuitVkHashes,
  readVkRecursiveHash,
} from "../scripts/utils";
import type { BfvDecryptionVerifier, BfvPkVerifier } from "../types";

const { ethers, networkHelpers } = await network.connect();
const { loadFixture } = networkHelpers;

const INTEGRATION_SUMMARY = path.join(
  REPO_ROOT,
  "circuits/benchmarks/results_insecure/integration_summary.json",
);

const hasCompiledVkArtifacts = (): boolean =>
  Object.values(BFV_PK_SUB_CIRCUIT_VK_HASH_PATHS).every((p) =>
    fs.existsSync(p),
  ) &&
  Object.values(BFV_DECRYPTION_SUB_CIRCUIT_VK_HASH_PATHS).every((p) =>
    fs.existsSync(p),
  );

const describeDeployTimeVkChecks = hasCompiledVkArtifacts()
  ? describe
  : describe.skip;

function hexToBytes32Array(hex: string): string[] {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  const out: string[] = [];
  for (let i = 0; i < clean.length; i += 64) {
    out.push(`0x${clean.slice(i, i + 64)}`);
  }
  return out;
}

function plaintextHashFromPublicInputs(publicInputs: string[]): string {
  const messageCoeffsCount = 100;
  const offset = publicInputs.length - messageCoeffsCount;
  const plaintext = new Uint8Array(messageCoeffsCount * 8);
  for (let i = 0; i < messageCoeffsCount; i++) {
    const coeff = BigInt(publicInputs[offset + i]);
    for (let j = 0; j < 8; j++) {
      plaintext[i * 8 + j] = Number((coeff >> BigInt(j * 8)) & 0xffn);
    }
  }
  return ethers.keccak256(plaintext);
}

describe("BfvVkBindingIntegration", function () {
  const deployHonkAndBfv = async () => {
    const libFactory = await ethers.getContractFactory(
      "contracts/verifiers/bfv/honk/DkgAggregatorVerifier.sol:ZKTranscriptLib",
    );
    const zkTranscriptLib = await libFactory.deploy();
    await zkTranscriptLib.waitForDeployment();
    const zkTranscriptLibAddress = await zkTranscriptLib.getAddress();

    const dkgAggFactory = await ethers.getContractFactory(
      "DkgAggregatorVerifier",
      {
        libraries: {
          "project/contracts/verifiers/bfv/honk/DkgAggregatorVerifier.sol:ZKTranscriptLib":
            zkTranscriptLibAddress,
        },
      },
    );
    const dkgAgg = await dkgAggFactory.deploy();
    await dkgAgg.waitForDeployment();

    const decAggFactory = await ethers.getContractFactory(
      "DecryptionAggregatorVerifier",
      {
        libraries: {
          "project/contracts/verifiers/bfv/honk/DecryptionAggregatorVerifier.sol:ZKTranscriptLib":
            zkTranscriptLibAddress,
        },
      },
    );
    const decAgg = await decAggFactory.deploy();
    await decAgg.waitForDeployment();

    const expectedNodesFoldKeyHash = readVkRecursiveHash(
      BFV_PK_SUB_CIRCUIT_VK_HASH_PATHS.nodesFold,
    );
    const expectedC5KeyHash = readVkRecursiveHash(
      BFV_PK_SUB_CIRCUIT_VK_HASH_PATHS.c5,
    );
    const expectedC6FoldKeyHash = readVkRecursiveHash(
      BFV_DECRYPTION_SUB_CIRCUIT_VK_HASH_PATHS.c6Fold,
    );
    const expectedC7KeyHash = readVkRecursiveHash(
      BFV_DECRYPTION_SUB_CIRCUIT_VK_HASH_PATHS.c7,
    );

    const bfvPk = await (
      await ethers.getContractFactory("BfvPkVerifier")
    ).deploy(
      await dkgAgg.getAddress(),
      expectedNodesFoldKeyHash,
      expectedC5KeyHash,
    );
    await bfvPk.waitForDeployment();

    const bfvDec = await (
      await ethers.getContractFactory("BfvDecryptionVerifier")
    ).deploy(
      await decAgg.getAddress(),
      expectedC6FoldKeyHash,
      expectedC7KeyHash,
    );
    await bfvDec.waitForDeployment();

    return {
      bfvPk: bfvPk as unknown as BfvPkVerifier,
      bfvDec: bfvDec as unknown as BfvDecryptionVerifier,
    };
  };

  describeDeployTimeVkChecks("deploy-time VK staleness checks", function () {
    it("rejects BfvPkVerifier with stale immutables", async function () {
      const { bfvPk } = await loadFixture(deployHonkAndBfv);
      const address = await bfvPk.getAddress();
      const stale = await (
        await ethers.getContractFactory("BfvPkVerifier")
      ).deploy(
        await bfvPk.circuitVerifier(),
        ethers.id("stale-nodes-fold"),
        ethers.id("stale-c5"),
      );
      await stale.waitForDeployment();

      await expect(
        assertBfvPkVerifierSubCircuitVkHashes(
          stale as unknown as BfvPkVerifier,
          await stale.getAddress(),
        ),
      ).to.be.rejectedWith(/stale sub-circuit VK immutables/);

      await expect(assertBfvPkVerifierSubCircuitVkHashes(bfvPk, address)).to.not
        .be.rejected;
    });

    it("rejects BfvDecryptionVerifier with stale immutables", async function () {
      const { bfvDec } = await loadFixture(deployHonkAndBfv);
      const address = await bfvDec.getAddress();
      const stale = await (
        await ethers.getContractFactory("BfvDecryptionVerifier")
      ).deploy(
        await bfvDec.circuitVerifier(),
        ethers.id("stale-c6"),
        ethers.id("stale-c7"),
      );
      await stale.waitForDeployment();

      await expect(
        assertBfvDecryptionVerifierSubCircuitVkHashes(
          stale as unknown as BfvDecryptionVerifier,
          await stale.getAddress(),
        ),
      ).to.be.rejectedWith(/stale sub-circuit VK immutables/);

      await expect(
        assertBfvDecryptionVerifierSubCircuitVkHashes(bfvDec, address),
      ).to.not.be.rejected;
    });
  });

  const runFoldedProofIntegration =
    fs.existsSync(INTEGRATION_SUMMARY) && hasCompiledVkArtifacts();

  (runFoldedProofIntegration ? it : it.skip)(
    "folded aggregator proofs: artifact VK hashes match publicInputs[0..1] and verify passes",
    async function () {
      this.timeout(120_000);

      const summary = JSON.parse(
        fs.readFileSync(INTEGRATION_SUMMARY, "utf8"),
      ) as {
        folded_artifacts: {
          dkg_aggregator: { proof_hex: string; public_inputs_hex: string };
          decryption_aggregator: {
            proof_hex: string;
            public_inputs_hex: string;
          };
        };
      };

      const dkgPublicInputs = hexToBytes32Array(
        summary.folded_artifacts.dkg_aggregator.public_inputs_hex,
      );
      const decPublicInputs = hexToBytes32Array(
        summary.folded_artifacts.decryption_aggregator.public_inputs_hex,
      );

      const expectedNodesFoldKeyHash = readVkRecursiveHash(
        BFV_PK_SUB_CIRCUIT_VK_HASH_PATHS.nodesFold,
      );
      const expectedC5KeyHash = readVkRecursiveHash(
        BFV_PK_SUB_CIRCUIT_VK_HASH_PATHS.c5,
      );
      const expectedC6FoldKeyHash = readVkRecursiveHash(
        BFV_DECRYPTION_SUB_CIRCUIT_VK_HASH_PATHS.c6Fold,
      );
      const expectedC7KeyHash = readVkRecursiveHash(
        BFV_DECRYPTION_SUB_CIRCUIT_VK_HASH_PATHS.c7,
      );

      expect(dkgPublicInputs[0]).to.equal(expectedNodesFoldKeyHash);
      expect(dkgPublicInputs[1]).to.equal(expectedC5KeyHash);
      expect(decPublicInputs[0]).to.equal(expectedC6FoldKeyHash);
      expect(decPublicInputs[1]).to.equal(expectedC7KeyHash);

      const { bfvPk, bfvDec } = await deployHonkAndBfv();
      const abiCoder = ethers.AbiCoder.defaultAbiCoder();

      const dkgEncoded = abiCoder.encode(
        ["bytes", "bytes32[]"],
        [summary.folded_artifacts.dkg_aggregator.proof_hex, dkgPublicInputs],
      );

      // Call the underlying circuit verifier directly — domain binding enforcement
      // in BfvPkVerifier requires circuit-side support (C-08 future work).
      const dkgCircuit = await ethers.getContractAt(
        "ICircuitVerifier",
        await bfvPk.circuitVerifier(),
      );
      const [rawDkgProof, rawDkgPi] = abiCoder.decode(
        ["bytes", "bytes32[]"],
        dkgEncoded,
      ) as [string, string[]];
      expect(
        await dkgCircuit.verify.staticCall(rawDkgProof, [...rawDkgPi]),
      ).to.equal(true);

      const decEncoded = abiCoder.encode(
        ["bytes", "bytes32[]"],
        [
          summary.folded_artifacts.decryption_aggregator.proof_hex,
          decPublicInputs,
        ],
      );
      const decCircuit = await ethers.getContractAt(
        "ICircuitVerifier",
        await bfvDec.circuitVerifier(),
      );
      const [rawDecProof, rawDecPi] = abiCoder.decode(
        ["bytes", "bytes32[]"],
        decEncoded,
      ) as [string, string[]];
      expect(
        await decCircuit.verify.staticCall(rawDecProof, [...rawDecPi]),
      ).to.equal(true);
    },
  );

  (runFoldedProofIntegration ? it : it.skip)(
    "rejects verify when expectedNodesFoldKeyHash is wrong by one byte",
    async function () {
      this.timeout(120_000);

      const summary = JSON.parse(
        fs.readFileSync(INTEGRATION_SUMMARY, "utf8"),
      ) as {
        folded_artifacts: {
          dkg_aggregator: { proof_hex: string; public_inputs_hex: string };
        };
      };

      const dkgPublicInputs = hexToBytes32Array(
        summary.folded_artifacts.dkg_aggregator.public_inputs_hex,
      );
      const expectedC5KeyHash = readVkRecursiveHash(
        BFV_PK_SUB_CIRCUIT_VK_HASH_PATHS.c5,
      );

      const libFactory = await ethers.getContractFactory(
        "contracts/verifiers/bfv/honk/DkgAggregatorVerifier.sol:ZKTranscriptLib",
      );
      const zkTranscriptLib = await libFactory.deploy();
      await zkTranscriptLib.waitForDeployment();

      const dkgAgg = await (
        await ethers.getContractFactory("DkgAggregatorVerifier", {
          libraries: {
            "project/contracts/verifiers/bfv/honk/DkgAggregatorVerifier.sol:ZKTranscriptLib":
              await zkTranscriptLib.getAddress(),
          },
        })
      ).deploy();
      await dkgAgg.waitForDeployment();

      const nodesFoldBuf = Buffer.from(dkgPublicInputs[0].slice(2), "hex");
      nodesFoldBuf[0] ^= 0xff;
      const wrongNodesFold = `0x${nodesFoldBuf.toString("hex")}`;

      const bfvPk = await (
        await ethers.getContractFactory("BfvPkVerifier")
      ).deploy(await dkgAgg.getAddress(), wrongNodesFold, expectedC5KeyHash);
      await bfvPk.waitForDeployment();

      const abiCoder = ethers.AbiCoder.defaultAbiCoder();
      const dkgEncoded = abiCoder.encode(
        ["bytes", "bytes32[]"],
        [summary.folded_artifacts.dkg_aggregator.proof_hex, dkgPublicInputs],
      );
      const pkCommitment = dkgPublicInputs[dkgPublicInputs.length - 1];

      // VkHashMismatch fires before the domain binding check, so dummy e3Id/root/nodes are fine.
      await expect(
        bfvPk.verify.staticCall(0n, 0n, [], pkCommitment, dkgEncoded),
      ).to.be.revertedWithCustomError(bfvPk, "VkHashMismatch");
    },
  );
});
