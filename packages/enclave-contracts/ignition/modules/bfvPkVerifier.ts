// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

import thresholdPkAggregationVerifierModule from "./thresholdPkAggregationVerifier";

export default buildModule("BfvPkVerifier", (m) => {
  const { thresholdPkAggregationVerifier } = m.useModule(
    thresholdPkAggregationVerifierModule,
  );

  const bfvPkVerifier = m.contract("BfvPkVerifier", [
    thresholdPkAggregationVerifier,
  ]);

  return { bfvPkVerifier };
}) as any;
