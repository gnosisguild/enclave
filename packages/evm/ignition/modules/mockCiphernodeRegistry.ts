import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("MockCiphernodeRegistry", (m) => {
  const mockCiphernodeRegistry = m.contract("MockCiphernodeRegistry");

  return { mockCiphernodeRegistry };
}) as any;