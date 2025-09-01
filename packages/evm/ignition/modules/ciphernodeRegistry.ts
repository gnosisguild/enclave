import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("CiphernodeRegistry", (m) => {
  const enclaveAddress = m.getParameter("enclaveAddress");
  const owner = m.getParameter("owner");

  const poseidonT3 = m.library("PoseidonT3");

  const cipherNodeRegistry = m.contract(
    "CiphernodeRegistryOwnable",
    [owner, enclaveAddress],
    {
      libraries: {
        PoseidonT3: poseidonT3,
      },
    },
  );

  return { cipherNodeRegistry };
}) as any;
