// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("Enclave", (m) => {
  const params = m.getParameter("params");
  const owner = m.getParameter("owner");
  const maxDuration = m.getParameter("maxDuration");
  const registry = m.getParameter("registry");
  const bondingRegistry = m.getParameter("bondingRegistry");
  const e3RefundManager = m.getParameter("e3RefundManager");
  const feeToken = m.getParameter("feeToken");
  const timeoutConfig = m.getParameter("timeoutConfig", {
    committeeFormationWindow: 3600,
    dkgWindow: 7200,
    computeWindow: 86400,
    decryptionWindow: 3600,
    gracePeriod: 600,
  });

  const enclaveImpl = m.contract("Enclave", []);

  const initData = m.encodeFunctionCall(enclaveImpl, "initialize", [
    owner,
    registry,
    bondingRegistry,
    e3RefundManager,
    feeToken,
    maxDuration,
    timeoutConfig,
    [params],
  ]);

  const enclave = m.contract("TransparentUpgradeableProxy", [
    enclaveImpl,
    owner,
    initData,
  ]);

  return { enclave };
}) as any;
