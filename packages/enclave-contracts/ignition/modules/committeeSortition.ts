import { buildModule } from "@nomicfoundation/hardhat-ignition/modules";

export default buildModule("CommitteeSortition", (m) => {
  const bondingRegistry = m.getParameter("bondingRegistry");
  const ciphernodeRegistry = m.getParameter("ciphernodeRegistry");

  // TODO: 5 minutes is the default submission window
  const submissionWindow = m.getParameter("submissionWindow", 300);

  const committeeSortition = m.contract("CommitteeSortition", [
    bondingRegistry,
    ciphernodeRegistry,
    submissionWindow,
  ]);

  return { committeeSortition };
}) as any;
