// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("E3RefundManager", (m) => {
  const owner = m.getParameter("owner");
  const enclave = m.getParameter("enclave");
  const e3Lifecycle = m.getParameter("e3Lifecycle");
  const feeToken = m.getParameter("feeToken");
  const bondingRegistry = m.getParameter("bondingRegistry");
  const treasury = m.getParameter("treasury");

  const e3RefundManagerImpl = m.contract("E3RefundManager", []);

  const initData = m.encodeFunctionCall(e3RefundManagerImpl, "initialize", [
    owner,
    enclave,
    e3Lifecycle,
    feeToken,
    bondingRegistry,
    treasury,
  ]);

  const e3RefundManager = m.contract("TransparentUpgradeableProxy", [
    e3RefundManagerImpl,
    owner,
    initData,
  ]);

  return { e3RefundManager };
}) as any;
