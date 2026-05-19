// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

import {
  BFV_DECRYPTION_SUB_CIRCUIT_VK_HASH_PATHS,
  readVkRecursiveHash,
} from "../../scripts/utils";
import decryptionAggregatorVerifierModule from "./decryptionAggregatorVerifier";

export default buildModule("BfvDecryptionVerifier", (m) => {
  const { decryptionAggregatorVerifier } = m.useModule(
    decryptionAggregatorVerifierModule,
  );

  const expectedC6FoldKeyHash = readVkRecursiveHash(
    BFV_DECRYPTION_SUB_CIRCUIT_VK_HASH_PATHS.c6Fold,
  );
  const expectedC7KeyHash = readVkRecursiveHash(
    BFV_DECRYPTION_SUB_CIRCUIT_VK_HASH_PATHS.c7,
  );

  const bfvDecryptionVerifier = m.contract("BfvDecryptionVerifier", [
    decryptionAggregatorVerifier,
    expectedC6FoldKeyHash,
    expectedC7KeyHash,
  ]);

  return { bfvDecryptionVerifier };
}) as any;
