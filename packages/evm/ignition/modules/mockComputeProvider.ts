import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("MockComputeProvider", (m) => {
  const mockComputeProvider = m.contract("MockComputeProvider");

  return { mockComputeProvider };
}) as any;
