import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("MockCiphernodeRegistryEmptyKey", (m) => {
  const mockCiphernodeRegistryEmptyKey = m.contract(
    "MockCiphernodeRegistryEmptyKey",
  );

  return { mockCiphernodeRegistryEmptyKey };
}) as any;
