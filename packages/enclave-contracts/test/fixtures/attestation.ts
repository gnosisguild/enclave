// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
// Shared attestation helpers for committee-based slashing tests.
import type { Signer } from "ethers";
import { network } from "hardhat";

const { ethers } = await network.connect();

const abiCoder = ethers.AbiCoder.defaultAbiCoder();

/// Canonical EIP-712 struct typehash for vote sigs (matches SlashingManager.VOTE_TYPEHASH).
export const VOTE_TYPEHASH = ethers.keccak256(
  ethers.toUtf8Bytes(
    "AccusationVote(uint256 e3Id,bytes32 accusationId,address voter,bool agrees,bytes32 dataHash)",
  ),
);

const VOTE_DOMAIN_NAME = "EnclaveSlashingManager";
const VOTE_DOMAIN_VERSION = "1";

/**
 * Helper to create signed committee attestation evidence for Lane A.
 * Each voter signs the canonical EIP-712 typed-data hash matching
 * `SlashingManager._verifyVotes`.
 * Returns
 *   abi.encode(proofType, voters, agrees, dataHashes, signatures, evidence)
 * with voters sorted ascending by address.
 *
 * `evidence` is the preimage of `dataHash` (the SlashingManager contract enforces
 * `keccak256(evidence) == dataHash`). If `evidence` is provided, `dataHash` is
 * derived from it automatically; pass `dataHash` explicitly only to test the
 * keccak-binding check itself.
 *
 * `slashingManager` is the deployed SlashingManager address — used as the
 * EIP-712 `verifyingContract`.
 */
export async function signAndEncodeAttestation(
  voterSigners: Signer[],
  e3Id: number,
  operator: string,
  slashingManager: string,
  proofType: number = 0,
  chainId: number = 31337,
  dataHash?: string,
  agreesOverride?: boolean[],
  evidence: string = "0x",
): Promise<string> {
  if (dataHash === undefined) {
    dataHash = ethers.keccak256(evidence);
  }
  const accusationId = ethers.keccak256(
    ethers.solidityPacked(
      ["uint256", "uint256", "address", "uint256"],
      [chainId, e3Id, operator, proofType],
    ),
  );

  const signersWithAddrs = await Promise.all(
    voterSigners.map(async (s, idx) => ({
      signer: s,
      address: await s.getAddress(),
      originalIndex: idx,
    })),
  );
  signersWithAddrs.sort((a, b) =>
    a.address.toLowerCase() < b.address.toLowerCase()
      ? -1
      : a.address.toLowerCase() > b.address.toLowerCase()
        ? 1
        : 0,
  );

  const voters: string[] = [];
  const agrees: boolean[] = [];
  const dataHashes: string[] = [];
  const signatures: string[] = [];

  const domain = {
    name: VOTE_DOMAIN_NAME,
    version: VOTE_DOMAIN_VERSION,
    chainId,
    verifyingContract: slashingManager,
  };
  const types = {
    AccusationVote: [
      { name: "e3Id", type: "uint256" },
      { name: "accusationId", type: "bytes32" },
      { name: "voter", type: "address" },
      { name: "agrees", type: "bool" },
      { name: "dataHash", type: "bytes32" },
    ],
  };

  for (let i = 0; i < signersWithAddrs.length; i++) {
    const {
      signer,
      address: voterAddress,
      originalIndex,
    } = signersWithAddrs[i]!;
    const voteAgrees =
      agreesOverride !== undefined ? agreesOverride[originalIndex]! : true;

    voters.push(voterAddress);
    agrees.push(voteAgrees);
    dataHashes.push(dataHash);

    const value = {
      e3Id,
      accusationId,
      voter: voterAddress,
      agrees: voteAgrees,
      dataHash,
    };
    const signature = await signer.signTypedData(domain, types, value);
    signatures.push(signature);
  }

  return abiCoder.encode(
    ["uint256", "address[]", "bool[]", "bytes32[]", "bytes[]", "bytes"],
    [proofType, voters, agrees, dataHashes, signatures, evidence],
  );
}
