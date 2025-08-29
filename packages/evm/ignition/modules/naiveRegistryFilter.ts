import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("NaiveRegistryFilter", (m) => {
  const ciphernodeRegistryAddress = m.getParameter("ciphernodeRegistryAddress");
  const owner = m.getParameter("owner");

  const naiveRegistryFilter = m.contract("NaiveRegistryFilter", [
    owner,
    ciphernodeRegistryAddress,
  ]);

  return { naiveRegistryFilter };
}) as any;
