// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/* eslint-disable @typescript-eslint/no-explicit-any */
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("BondingRegistry", (m) => {
  const ticketToken = m.getParameter("ticketToken");
  const licenseToken = m.getParameter("licenseToken");
  const registry = m.getParameter("registry");
  const slashedFundsTreasury = m.getParameter("slashedFundsTreasury");
  const ticketPrice = m.getParameter("ticketPrice");
  const licenseRequiredBond = m.getParameter("licenseRequiredBond");
  const minTicketBalance = m.getParameter("minTicketBalance");
  const exitDelay = m.getParameter("exitDelay");
  const owner = m.getParameter("owner");

  const bondingRegistry = m.contract("BondingRegistry", [
    owner,
    ticketToken,
    licenseToken,
    registry,
    slashedFundsTreasury,
    ticketPrice,
    licenseRequiredBond,
    minTicketBalance,
    exitDelay,
  ]);

  return { bondingRegistry };
}) as any;
