import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("MockDecryptionVerifier", (m) => {
  const mockDecryptionVerifier = m.contract("MockDecryptionVerifier");

  return { mockDecryptionVerifier };
}) as any;
