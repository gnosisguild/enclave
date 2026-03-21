// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("RecursiveAggregationFoldVerifier", (m) => {
  const recursiveAggregationFoldVerifier = m.contract(
    "RecursiveAggregationFoldVerifier",
  );

  return { recursiveAggregationFoldVerifier };
}) as any;
