// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("E3Lifecycle", (m) => {
  const owner = m.getParameter("owner");
  const enclave = m.getParameter("enclave");
  const committeeFormationWindow = m.getParameter("committeeFormationWindow");
  const dkgWindow = m.getParameter("dkgWindow");
  const computeWindow = m.getParameter("computeWindow");
  const decryptionWindow = m.getParameter("decryptionWindow");
  const gracePeriod = m.getParameter("gracePeriod");

  const e3LifecycleImpl = m.contract("E3Lifecycle", []);

  const initData = m.encodeFunctionCall(e3LifecycleImpl, "initialize", [
    owner,
    enclave,
    {
      committeeFormationWindow,
      dkgWindow,
      computeWindow,
      decryptionWindow,
      gracePeriod,
    },
  ]);

  const e3Lifecycle = m.contract("TransparentUpgradeableProxy", [
    e3LifecycleImpl,
    owner,
    initData,
  ]);

  return { e3Lifecycle };
}) as any;
