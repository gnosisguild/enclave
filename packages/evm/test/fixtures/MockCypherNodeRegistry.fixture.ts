import { ethers } from "hardhat";

import { MockCypherNodeRegistry } from "../../types/contracts/test/MockCypherNodeRegistry";
import { MockCypherNodeRegistry__factory } from "../../types/factories/contracts/test/MockCypherNodeRegistry__factory";

export async function deployMockCypherNodeRegistryFixture() {
  const MockCypherNodeRegistry = (await ethers.getContractFactory(
    "MockCypherNodeRegistry",
  )) as MockCypherNodeRegistry__factory;
  const mockCypherNodeRegistry = (await MockCypherNodeRegistry.deploy()) as MockCypherNodeRegistry;
  const mockCypherNodeRegistry_address = await mockCypherNodeRegistry.getAddress();

  return { mockCypherNodeRegistry, mockCypherNodeRegistry_address };
}
