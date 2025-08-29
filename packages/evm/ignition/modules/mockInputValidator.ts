import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("MockInputValidator", (m) => {
  const mockInputValidator = m.contract("MockInputValidator");

  return { mockInputValidator };
}) as any;
