// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/* eslint-disable @typescript-eslint/no-explicit-any */
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("Enclave", (m) => {
  const params = m.getParameter("params");
  const owner = m.getParameter("owner");
  const maxDuration = m.getParameter("maxDuration");
  const registry = m.getParameter("registry");
  const bondingRegistry = m.getParameter("bondingRegistry");
  const usdcToken = m.getParameter("usdcToken");

  const poseidonT3 = m.library("PoseidonT3");

  const enclave = m.contract(
    "Enclave",
    [owner, registry, bondingRegistry, usdcToken, maxDuration, [params]],
    {
      libraries: {
        PoseidonT3: poseidonT3,
      },
    },
  );

  return { enclave };
}) as any;
