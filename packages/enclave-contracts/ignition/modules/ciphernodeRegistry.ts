// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/* eslint-disable @typescript-eslint/no-explicit-any */
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("CiphernodeRegistry", (m) => {
  const enclaveAddress = m.getParameter("enclaveAddress");
  const owner = m.getParameter("owner");
  const submissionWindow = m.getParameter("submissionWindow");

  const poseidonT3 = m.library("PoseidonT3");

  const ciphernodeRegistryImpl = m.contract("CiphernodeRegistryOwnable", [], {
    libraries: {
      PoseidonT3: poseidonT3,
    },
  });

  const initData = m.encodeFunctionCall(ciphernodeRegistryImpl, "initialize", [
    owner,
    enclaveAddress,
    submissionWindow,
  ]);

  const ciphernodeRegistry = m.contract("TransparentUpgradeableProxy", [
    ciphernodeRegistryImpl,
    owner,
    initData,
  ]);

  return { ciphernodeRegistry, ciphernodeRegistryImpl, poseidonT3 };
}) as any;
