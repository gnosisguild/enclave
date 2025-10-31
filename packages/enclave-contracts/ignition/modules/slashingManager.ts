// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/* eslint-disable @typescript-eslint/no-explicit-any */
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("SlashingManager", (m) => {
  const bondingRegistry = m.getParameter("bondingRegistry");
  const admin = m.getParameter("admin");

  const slashingManager = m.contract("SlashingManager", [
    admin,
    bondingRegistry,
  ]);

  return { slashingManager };
}) as any;
