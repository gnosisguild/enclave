// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";
import fs from "node:fs";
import path from "node:path";

import {
  BFV_DECRYPTION_SUB_CIRCUIT_VK_HASH_PATHS,
  BFV_DKG_H,
  BFV_PK_SUB_CIRCUIT_VK_HASH_PATHS,
  BFV_THRESHOLD_T,
  committeeHashFromLimbs,
  readVkRecursiveHash,
} from "./utils";

function findRawJson(rawDir: string, fragment: string): any {
  const entries = fs.readdirSync(rawDir).filter((f) => f.endsWith(".json"));
  for (const f of entries) {
    if (!f.includes(fragment)) continue;
    const full = path.join(rawDir, f);
    return JSON.parse(fs.readFileSync(full, "utf8"));
  }
  throw new Error(`Missing raw benchmark JSON for fragment: ${fragment}`);
}

const MIN_VK_HASH_PUBLIC_INPUTS = 2;
const DKG_COMMITTEE_HASH_IDX_HI = 5;
const DKG_COMMITTEE_HASH_IDX_LO = 6;
const DEC_COMMITTEE_HASH_IDX_HI = 2;
const DEC_COMMITTEE_HASH_IDX_LO = 3;

function requirePublicInputLen(
  label: string,
  publicInputs: string[],
  minLen: number,
): void {
  if (publicInputs.length < minLen) {
    throw new Error(
      `${label}: public_inputs length ${publicInputs.length} < ${minLen} (truncated or stale artifact?)`,
    );
  }
}

function hexToBytes32Array(hex: string): string[] {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  if (clean.length === 0) return [];
  if (clean.length % 64 !== 0) {
    throw new Error(
      `public_inputs_hex length is not 32-byte aligned: ${clean.length}`,
    );
  }
  const out: string[] = [];
  for (let i = 0; i < clean.length; i += 64) {
    out.push(`0x${clean.slice(i, i + 64)}`);
  }
  return out;
}

function plaintextHashFromPublicInputs(
  publicInputs: string[],
  ethersLib: any,
): string {
  const messageCoeffsCount = 100;
  if (publicInputs.length < messageCoeffsCount) {
    throw new Error(`Not enough public inputs: ${publicInputs.length}`);
  }
  const offset = publicInputs.length - messageCoeffsCount;
  const plaintext = new Uint8Array(messageCoeffsCount * 8);
  for (let i = 0; i < messageCoeffsCount; i++) {
    const coeff = BigInt(publicInputs[offset + i]);
    for (let j = 0; j < 8; j++) {
      plaintext[i * 8 + j] = Number((coeff >> BigInt(j * 8)) & 0xffn);
    }
  }
  return ethersLib.keccak256(plaintext);
}

async function main() {
  const rawDir = process.env.BENCHMARK_RAW_DIR;
  const outputPath = process.env.BENCHMARK_GAS_OUTPUT;
  const foldedPath = process.env.BENCHMARK_FOLDED_JSON;
  if (!rawDir || !outputPath) {
    throw new Error("BENCHMARK_RAW_DIR and BENCHMARK_GAS_OUTPUT are required");
  }

  const { ethers } = await network.connect();

  let dkgProofHex: string | undefined;
  let dkgPublicHex: string | undefined;
  let decProofHex: string | undefined;
  let decPublicHex: string | undefined;

  if (foldedPath && fs.existsSync(foldedPath)) {
    const folded = JSON.parse(fs.readFileSync(foldedPath, "utf8"));
    dkgProofHex = folded?.dkg_aggregator?.proof_hex;
    dkgPublicHex = folded?.dkg_aggregator?.public_inputs_hex;
    decProofHex = folded?.decryption_aggregator?.proof_hex;
    decPublicHex = folded?.decryption_aggregator?.public_inputs_hex;
  } else {
    const dkgRaw = findRawJson(rawDir, "threshold_pk_aggregation");
    const decRaw = findRawJson(
      rawDir,
      "threshold_decrypted_shares_aggregation",
    );
    dkgProofHex = dkgRaw?.proof_generation?.proof_hex;
    dkgPublicHex = dkgRaw?.verification?.public_inputs_hex;
    decProofHex = decRaw?.proof_generation?.proof_hex;
    decPublicHex = decRaw?.verification?.public_inputs_hex;
  }

  if (!dkgProofHex || !dkgPublicHex || !decProofHex || !decPublicHex) {
    throw new Error(
      "Missing proof/public_inputs hex fields in raw benchmark JSON",
    );
  }

  const dkgPublicInputs = hexToBytes32Array(dkgPublicHex);
  const decPublicInputs = hexToBytes32Array(decPublicHex);
  requirePublicInputLen(
    "dkg_aggregator",
    dkgPublicInputs,
    MIN_VK_HASH_PUBLIC_INPUTS,
  );
  requirePublicInputLen(
    "decryption_aggregator",
    decPublicInputs,
    MIN_VK_HASH_PUBLIC_INPUTS,
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

  if (
    dkgPublicInputs[0] !== expectedNodesFoldKeyHash ||
    dkgPublicInputs[1] !== expectedC5KeyHash
  ) {
    throw new Error(
      "DKG aggregator proof publicInputs[0..1] do not match nodes_fold / pk_aggregation .vk_recursive_hash artifacts",
    );
  }
  if (
    decPublicInputs[0] !== expectedC6FoldKeyHash ||
    decPublicInputs[1] !== expectedC7KeyHash
  ) {
    throw new Error(
      "Decryption aggregator proof publicInputs[0..1] do not match c6_fold / decrypted_shares_aggregation .vk_recursive_hash artifacts",
    );
  }

  const abiCoder = ethers.AbiCoder.defaultAbiCoder();

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
  const dkgAggAddress = await dkgAgg.getAddress();

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
  const decAggAddress = await decAgg.getAddress();

  const bfvPk = await (
    await ethers.getContractFactory("BfvPkVerifier")
  ).deploy(
    dkgAggAddress,
    expectedNodesFoldKeyHash,
    expectedC5KeyHash,
    BFV_DKG_H,
  );
  await bfvPk.waitForDeployment();

  const dkgEncodedProof = abiCoder.encode(
    ["bytes", "bytes32[]"],
    [dkgProofHex, dkgPublicInputs],
  );
  requirePublicInputLen(
    "dkg_aggregator committee_hash",
    dkgPublicInputs,
    DKG_COMMITTEE_HASH_IDX_LO + 1,
  );
  const pkCommitment = dkgPublicInputs[dkgPublicInputs.length - 1];
  const dkgCommitteeHash = committeeHashFromLimbs(
    dkgPublicInputs[DKG_COMMITTEE_HASH_IDX_HI],
    dkgPublicInputs[DKG_COMMITTEE_HASH_IDX_LO],
  );
  const dkgOk = await bfvPk.verify.staticCall(
    pkCommitment,
    dkgCommitteeHash,
    dkgEncodedProof,
  );
  if (!dkgOk) {
    throw new Error(
      "BfvPkVerifier.verify returned false for folded DKG proof (Honk VK / proof mismatch?)",
    );
  }
  const dkgGas = await bfvPk.verify.estimateGas(
    pkCommitment,
    dkgCommitteeHash,
    dkgEncodedProof,
  );

  const bfvDec = await (
    await ethers.getContractFactory("BfvDecryptionVerifier")
  ).deploy(
    decAggAddress,
    expectedC6FoldKeyHash,
    expectedC7KeyHash,
    BFV_THRESHOLD_T,
  );
  await bfvDec.waitForDeployment();

  const decEncodedProof = abiCoder.encode(
    ["bytes", "bytes32[]"],
    [decProofHex, decPublicInputs],
  );
  const plaintextHash = plaintextHashFromPublicInputs(decPublicInputs, ethers);
  requirePublicInputLen(
    "decryption_aggregator committee_hash",
    decPublicInputs,
    DEC_COMMITTEE_HASH_IDX_LO + 1,
  );
  const decCommitteeHash = committeeHashFromLimbs(
    decPublicInputs[DEC_COMMITTEE_HASH_IDX_HI],
    decPublicInputs[DEC_COMMITTEE_HASH_IDX_LO],
  );
  const decOk = await bfvDec.verify.staticCall(
    plaintextHash,
    decCommitteeHash,
    decEncodedProof,
  );
  if (!decOk) {
    throw new Error(
      "BfvDecryptionVerifier.verify returned false for folded decryption proof (Honk VK / proof mismatch?)",
    );
  }
  const decGas = await bfvDec.verify.estimateGas(
    plaintextHash,
    decCommitteeHash,
    decEncodedProof,
  );

  const output = {
    verify_gas: {
      dkg: Number(dkgGas),
      dec: Number(decGas),
    },
    source: "benchmark_raw_artifacts",
  };
  fs.writeFileSync(outputPath, JSON.stringify(output, null, 2));
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
