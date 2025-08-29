import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("MockE3Program", (m) => {
  const mockInputValidator = m.getParameter("mockInputValidator", "address");

  const mockE3Program = m.contract("MockE3Program", [mockInputValidator]);

  return { mockE3Program };
}) as any;
