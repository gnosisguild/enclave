// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { Signer } from "ethers";

import { ethers } from "./connection";

export type DkgFoldAttestation = {
  partyId: number;
  skAggCommit: string;
  prfKeyCommitsHash: string;
  signature: string;
};

export type DkgPartySlotBinding = {
  partyId: number;
  node: string;
};

/** Public inputs layout expected by `DkgFoldAttestationVerifier` for honest count `h`. */
export function encodeMockDkgProofForAttestation(
  pkCommitment: string,
  partyIds: number[],
  skCommits: string[],
  prfCommits: string[],
  n: number,
): string {
  const h = partyIds.length;
  const publicInputs: string[] = Array.from(
    { length: 6 + 2 * h + h * n },
    () => ethers.ZeroHash,
  );
  publicInputs[5 + 2 * h] = pkCommitment;
  for (let i = 0; i < h; i++) {
    publicInputs[2 + i] = ethers.zeroPadValue(ethers.toBeHex(partyIds[i]), 32);
    publicInputs[5 + h + i] = skCommits[i];
  }
  prfCommits.forEach((key, i) => {
    publicInputs[6 + 2 * h + i] = key;
  });
  return ethers.AbiCoder.defaultAbiCoder().encode(
    ["bytes", "bytes32[]"],
    ["0x", publicInputs],
  );
}

/** Sign one EIP-712 fold-attestation tuple. */
export async function signFoldAttestation(
  signer: Signer,
  chainId: bigint,
  verifyingContract: string,
  registry: string,
  e3Id: number | bigint,
  partyId: number,
  skAggCommit: string,
  prfKeyCommitsHash: string,
): Promise<string> {
  const domain = {
    name: "InterfoldDkgFoldAttestation",
    version: "2",
    chainId,
    verifyingContract,
  };
  const types = {
    DkgFoldAttestation: [
      { name: "registry", type: "address" },
      { name: "e3Id", type: "uint256" },
      { name: "partyId", type: "uint256" },
      { name: "skAggCommit", type: "bytes32" },
      { name: "prfKeyCommitsHash", type: "bytes32" },
    ],
  };
  return signer.signTypedData(domain, types, {
    registry,
    e3Id,
    partyId,
    skAggCommit,
    prfKeyCommitsHash,
  });
}

/**
 * Build mock proof + bundle payloads for `publishCommittee`/verifier tests.
 * Operators are sorted by address to match on-chain canonical `topNodes`.
 */
export async function buildMockDkgAttestationFixtureData(
  operators: Signer[],
  e3Id: number | bigint,
  pkCommitment: string,
  signingVerifierAddress: string,
  signingRegistryAddress: string,
): Promise<{
  ordered: { op: Signer; addr: string }[];
  proof: string;
  bundle: string;
  partyIds: number[];
  skCommits: string[];
  prfCommits: string[];
  attestations: DkgFoldAttestation[];
  bindings: DkgPartySlotBinding[];
}> {
  const ordered = await Promise.all(
    operators.map(async (op) => ({ op, addr: await op.getAddress() })),
  );
  ordered.sort((a, b) =>
    a.addr.toLowerCase() < b.addr.toLowerCase()
      ? -1
      : a.addr.toLowerCase() > b.addr.toLowerCase()
        ? 1
        : 0,
  );

  const partyIds = ordered.map((_, idx) => idx);
  const skCommits = partyIds.map((i) => ethers.id(`sk-${e3Id}-${i}`));
  const n = ordered.length;
  const prfCommits = partyIds.flatMap((i) =>
    partyIds.map((j) => ethers.id(`prf-${e3Id}-${i}-${j}`)),
  );
  const proof = encodeMockDkgProofForAttestation(
    pkCommitment,
    partyIds,
    skCommits,
    prfCommits,
    n,
  );

  const { chainId } = await ethers.provider.getNetwork();
  const attestations: DkgFoldAttestation[] = [];
  const bindings: DkgPartySlotBinding[] = [];
  for (let i = 0; i < ordered.length; i++) {
    const prfKeyCommitsHash = ethers.keccak256(
      ethers.concat(prfCommits.slice(i * n, (i + 1) * n)),
    );
    attestations.push({
      partyId: i,
      skAggCommit: skCommits[i],
      prfKeyCommitsHash,
      signature: await signFoldAttestation(
        ordered[i].op,
        chainId,
        signingVerifierAddress,
        signingRegistryAddress,
        e3Id,
        i,
        skCommits[i],
        prfKeyCommitsHash,
      ),
    });
    bindings.push({ partyId: i, node: ordered[i].addr });
  }

  const bundle = ethers.AbiCoder.defaultAbiCoder().encode(
    [
      "tuple(uint256 partyId, bytes32 skAggCommit, bytes32 prfKeyCommitsHash, bytes signature)[]",
      "tuple(uint256 partyId, address node)[]",
    ],
    [attestations, bindings],
  );

  return {
    ordered,
    proof,
    bundle,
    partyIds,
    skCommits,
    prfCommits,
    attestations,
    bindings,
  };
}

/** Convenience helper for Interfold tests with a plaintext public key input. */
export async function buildMockAggregationPublishArgs(
  operators: Signer[],
  e3Id: number | bigint,
  publicKey: string,
  signingVerifierAddress: string,
  signingRegistryAddress: string,
): Promise<{ proof: string; bundle: string }> {
  const fixture = await buildMockDkgAttestationFixtureData(
    operators,
    e3Id,
    ethers.keccak256(publicKey),
    signingVerifierAddress,
    signingRegistryAddress,
  );
  return { proof: fixture.proof, bundle: fixture.bundle };
}
