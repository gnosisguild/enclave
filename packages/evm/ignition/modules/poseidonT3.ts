import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("PoseidonT3", (m) => {
  const poseidon = m.library("PoseidonT3");

  return { poseidon };
}) as any;
