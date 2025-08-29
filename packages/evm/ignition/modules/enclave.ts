import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("Enclave", (m) => {
  const params = m.getParameter("params");
  const owner = m.getParameter("owner");
  const maxDuration = m.getParameter("maxDuration");
  const registry = m.getParameter("registry");

  const poseidonT3 = m.library("PoseidonT3");

  const enclave = m.contract(
    "Enclave",
    [owner, registry, maxDuration, [params]],
    {
      libraries: {
        PoseidonT3: poseidonT3,
      },
    },
  );

  return { enclave };
}) as any;
