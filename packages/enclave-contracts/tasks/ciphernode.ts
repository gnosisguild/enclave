// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { LeanIMT } from "@zk-kit/lean-imt";
import { ZeroAddress } from "ethers";
import { task } from "hardhat/config";
import { poseidon2 } from "poseidon-lite";

export const ciphernodeAdd = task(
  "ciphernode:add",
  "Register a ciphernode to the registry",
)
  .addOption({
    name: "ciphernodeAddress",
    description: "address of ciphernode to register",
    defaultValue: ZeroAddress,
  })
  .setAction(async () => ({
    default: async ({ ciphernodeAddress }, hre) => {
      const { deployAndSaveCiphernodeRegistryOwnable } = await import(
        "../scripts/deployAndSave/ciphernodeRegistryOwnable"
      );
      const { ciphernodeRegistry } =
        await deployAndSaveCiphernodeRegistryOwnable({ hre });

      const tx = await ciphernodeRegistry.addCiphernode(ciphernodeAddress);
      await tx.wait();
      console.log(`Ciphernode ${ciphernodeAddress} registered`);
    },
  }))
  .build();

export const ciphernodeRemove = task(
  "ciphernode:remove",
  "Remove a ciphernode from the registry",
)
  .addOption({
    name: "ciphernodeAddress",
    description: "address of ciphernode to remove",
    defaultValue: ZeroAddress,
  })
  .addOption({
    name: "siblings",
    description: "comma separated siblings from tree proof",
    defaultValue: ZeroAddress,
  })
  .setAction(async () => ({
    default: async ({ ciphernodeAddress, siblings }, hre) => {
      const { deployAndSaveCiphernodeRegistryOwnable } = await import(
        "../scripts/deployAndSave/ciphernodeRegistryOwnable"
      );
      const { ciphernodeRegistry } =
        await deployAndSaveCiphernodeRegistryOwnable({ hre });

      const siblingsArray = siblings.split(",").map((s: string) => BigInt(s));

      const tx = await ciphernodeRegistry.removeCiphernode(
        ciphernodeAddress,
        siblingsArray,
      );
      await tx.wait();

      console.log(`Ciphernode ${ciphernodeAddress} removed`);
    },
  }))
  .build();

export const ciphernodeSiblings = task(
  "ciphernode:siblings",
  "Get the sibling of a ciphernode in the registry",
)
  .addOption({
    name: "ciphernodeAddress",
    description: "address of ciphernode to get siblings for",
    defaultValue: ZeroAddress,
  })
  .addOption({
    name: "ciphernodeAddresses",
    description:
      "comma separated addresses of ciphernodes in the order they were added to the registry",
    defaultValue: ZeroAddress,
  })
  .setAction(async () => ({
    default: async ({ ciphernodeAddress, ciphernodeAddresses }, _) => {
      const hash = (a: bigint, b: bigint) => poseidon2([a, b]);
      const tree = new LeanIMT(hash);

      const addresses = ciphernodeAddresses.split(",");

      for (const address of addresses) {
        tree.insert(BigInt(address));
      }

      const index = tree.indexOf(BigInt(ciphernodeAddress));
      const { siblings } = tree.generateProof(index);

      console.log(`Siblings for ${ciphernodeAddress}: ${siblings}`);
    },
  }))
  .build();
