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

  const bfvDecryptionVerifier = m.contract("BfvDecryptionVerifier", [
    decryptionAggregatorVerifier,
  ]);

  return { bfvDecryptionVerifier };
}) as any;
