// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

import { MockE3Program__factory } from "../../types";
import MockE3ProgramModule from "../../ignition/modules/mockE3Program";

const { ignition } = await network.connect();

export async function deployE3ProgramFixture(inputValidator: string) {
  const { mockE3Program } = await ignition.deploy(MockE3ProgramModule, {
    parameters: {
      MockE3Program: {
        mockInputValidator: inputValidator,
      },
    },
  });
  return MockE3Program__factory.connect(await mockE3Program.getAddress());
}
