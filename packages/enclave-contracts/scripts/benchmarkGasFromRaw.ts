// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";
import fs from "node:fs";
import path from "node:path";

function findRawJson(rawDir: string, fragment: string): any {
  const entries = fs.readdirSync(rawDir).filter((f) => f.endsWith(".json"));
  for (const f of entries) {
    if (!f.includes(fragment)) continue;
    const full = path.join(rawDir, f);
    return JSON.parse(fs.readFileSync(full, "utf8"));
  }
  throw new Error(`Missing raw benchmark JSON for fragment: ${fragment}`);
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
  ).deploy(dkgAggAddress);
  await bfvPk.waitForDeployment();

  const dkgEncodedProof = abiCoder.encode(
    ["bytes", "bytes32[]"],
    [dkgProofHex, dkgPublicInputs],
  );
  const pkCommitment = dkgPublicInputs[dkgPublicInputs.length - 1];
  const dkgGas = await bfvPk.verify.estimateGas(pkCommitment, dkgEncodedProof);

  const bfvDec = await (
    await ethers.getContractFactory("BfvDecryptionVerifier")
  ).deploy(decAggAddress);
  await bfvDec.waitForDeployment();

  const decEncodedProof = abiCoder.encode(
    ["bytes", "bytes32[]"],
    [decProofHex, decPublicInputs],
  );
  const plaintextHash = plaintextHashFromPublicInputs(decPublicInputs, ethers);
  const decGas = await bfvDec.verify.estimateGas(
    plaintextHash,
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
