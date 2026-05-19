// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("Enclave", (m) => {
  const owner = m.getParameter("owner");
  const maxDuration = m.getParameter("maxDuration");
  const registry = m.getParameter("registry");
  const bondingRegistry = m.getParameter("bondingRegistry");
  const e3RefundManager = m.getParameter("e3RefundManager");
  const feeToken = m.getParameter("feeToken");
  const timeoutConfig = m.getParameter("timeoutConfig", {
    dkgWindow: 7200,
    computeWindow: 86400,
    decryptionWindow: 3600,
  });

  // Pure pricing math is delegated to the EnclavePricing external library
  // (DELEGATECALL link) so the deployed Enclave runtime stays under the
  // EIP-170 24,576-byte cap.
  const enclavePricing = m.library("EnclavePricing");
  const enclaveImpl = m.contract("Enclave", [], {
    libraries: { EnclavePricing: enclavePricing },
  });

  const initData = m.encodeFunctionCall(enclaveImpl, "initialize", [
    owner,
    registry,
    bondingRegistry,
    e3RefundManager,
    feeToken,
    maxDuration,
    timeoutConfig,
  ]);

  const enclave = m.contract("TransparentUpgradeableProxy", [
    enclaveImpl,
    owner,
    initData,
  ]);

  return { enclave };
}) as any;
