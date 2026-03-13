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

export const VOTE_TYPEHASH = ethers.keccak256(
  ethers.toUtf8Bytes(
    "AccusationVote(uint256 chainId,uint256 e3Id,bytes32 accusationId,address voter,bool agrees,bytes32 dataHash)",
  ),
);

/**
 * Helper to create signed committee attestation evidence for Lane A.
 * Each voter signs a VOTE_TYPEHASH-structured digest via personal_sign (EIP-191).
 * Returns abi.encode(proofType, voters, agrees, dataHashes, signatures)
 * with voters sorted ascending by address.
 */
export async function signAndEncodeAttestation(
  voterSigners: Signer[],
  e3Id: number,
  operator: string,
  proofType: number = 0,
  chainId: number = 31337,
  dataHash: string = ethers.ZeroHash,
  agreesOverride?: boolean[],
): Promise<string> {
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

    const messageHash = ethers.keccak256(
      abiCoder.encode(
        [
          "bytes32",
          "uint256",
          "uint256",
          "bytes32",
          "address",
          "bool",
          "bytes32",
        ],
        [
          VOTE_TYPEHASH,
          chainId,
          e3Id,
          accusationId,
          voterAddress,
          voteAgrees,
          dataHash,
        ],
      ),
    );
    const signature = await signer.signMessage(ethers.getBytes(messageHash));
    signatures.push(signature);
  }

  return abiCoder.encode(
    ["uint256", "address[]", "bool[]", "bytes32[]", "bytes[]"],
    [proofType, voters, agrees, dataHashes, signatures],
  );
}
