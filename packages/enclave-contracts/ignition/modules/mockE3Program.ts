// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/* eslint-disable @typescript-eslint/no-explicit-any */
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("MockE3Program", (m) => {
  const mockInputValidator = m.getParameter("mockInputValidator");

  const mockE3Program = m.contract("MockE3Program", [mockInputValidator]);

  return { mockE3Program };
}) as any;
