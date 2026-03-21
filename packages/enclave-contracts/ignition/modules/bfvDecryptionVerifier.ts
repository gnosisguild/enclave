// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

import thresholdDecryptedSharesAggregationVerifierModule from "./thresholdDecryptedSharesAggregationVerifier";

export default buildModule("BfvDecryptionVerifier", (m) => {
  const { thresholdDecryptedSharesAggregationVerifier } = m.useModule(
    thresholdDecryptedSharesAggregationVerifierModule,
  );

  const bfvDecryptionVerifier = m.contract("BfvDecryptionVerifier", [
    thresholdDecryptedSharesAggregationVerifier,
  ]);

  return { bfvDecryptionVerifier };
}) as any;
