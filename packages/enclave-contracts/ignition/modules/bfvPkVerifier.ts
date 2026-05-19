// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

import dkgAggregatorVerifierModule from "./dkgAggregatorVerifier";

export default buildModule("BfvPkVerifier", (m) => {
  const { dkgAggregatorVerifier } = m.useModule(dkgAggregatorVerifierModule);

  // Recursive sub-circuit VK hashes (M-34 anchor) — pinned per deployment.
  // Provenance: `bb verify_key -b
  //   circuits/bin/recursive_aggregation/{node_fold,c5_pk_aggregation}/target/...`
  // committed to source so on-chain verification is reproducible. The defaults
  // here are placeholders (`bytes32(0)`) suitable for ignition test runs; real
  // deployments MUST override via module parameters.
  const nodesFoldKeyHash = m.getParameter(
    "nodesFoldKeyHash",
    "0x0000000000000000000000000000000000000000000000000000000000000000",
  );
  const c5KeyHash = m.getParameter(
    "c5KeyHash",
    "0x0000000000000000000000000000000000000000000000000000000000000000",
  );

  const bfvPkVerifier = m.contract("BfvPkVerifier", [
    dkgAggregatorVerifier,
    nodesFoldKeyHash,
    c5KeyHash,
  ]);

  return { bfvPkVerifier };
}) as any;
