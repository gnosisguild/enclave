// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

import recursiveAggregationFoldVerifierModule from "./recursiveAggregationFoldVerifier";
import thresholdPkAggregationVerifierModule from "./thresholdPkAggregationVerifier";

export default buildModule("BfvPkVerifier", (m) => {
  const { thresholdPkAggregationVerifier } = m.useModule(
    thresholdPkAggregationVerifierModule,
  );
  const { recursiveAggregationFoldVerifier } = m.useModule(
    recursiveAggregationFoldVerifierModule,
  );

  const bfvPkVerifier = m.contract("BfvPkVerifier", [
    thresholdPkAggregationVerifier,
    recursiveAggregationFoldVerifier,
  ]);

  return { bfvPkVerifier };
}) as any;
