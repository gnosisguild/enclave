// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/* eslint-disable @typescript-eslint/no-explicit-any */
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("NaiveRegistryFilter", (m) => {
  const ciphernodeRegistryAddress = m.getParameter("ciphernodeRegistryAddress");
  const owner = m.getParameter("owner");

  const naiveRegistryFilter = m.contract("NaiveRegistryFilter", [
    owner,
    ciphernodeRegistryAddress,
  ]);

  return { naiveRegistryFilter };
}) as any;
