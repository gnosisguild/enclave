// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

import decryptionAggregatorVerifierModule from "./decryptionAggregatorVerifier";

export default buildModule("BfvDecryptionVerifier", (m) => {
  const { decryptionAggregatorVerifier } = m.useModule(
    decryptionAggregatorVerifierModule,
  );

  // Recursive sub-circuit VK hashes (M-34 anchor) — pinned per deployment.
  // Provenance: `bb verify_key -b
  //   circuits/bin/recursive_aggregation/{c6_fold,c7_decrypted_shares_aggregation}/target/...`
  // committed to source so on-chain verification is reproducible. The defaults
  // here are placeholders (`bytes32(0)`) suitable for ignition test runs; real
  // deployments MUST override via module parameters.
  const c6FoldKeyHash = m.getParameter(
    "c6FoldKeyHash",
    "0x0000000000000000000000000000000000000000000000000000000000000000",
  );
  const c7KeyHash = m.getParameter(
    "c7KeyHash",
    "0x0000000000000000000000000000000000000000000000000000000000000000",
  );

  const bfvDecryptionVerifier = m.contract("BfvDecryptionVerifier", [
    decryptionAggregatorVerifier,
    c6FoldKeyHash,
    c7KeyHash,
  ]);

  return { bfvDecryptionVerifier };
}) as any;
