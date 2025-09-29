// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/* eslint-disable @typescript-eslint/no-explicit-any */
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("EnclaveTicketToken", (m) => {
  const underlyingUSDC = m.getParameter("underlyingUSDC");
  const registry = m.getParameter("registry");
  const owner = m.getParameter("owner");

  const enclaveTicketToken = m.contract("EnclaveTicketToken", [
    underlyingUSDC,
    registry,
    owner,
  ]);

  return { enclaveTicketToken };
}) as any;
