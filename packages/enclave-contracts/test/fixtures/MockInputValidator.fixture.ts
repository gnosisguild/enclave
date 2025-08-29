// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

import { MockInputValidator__factory } from "../../types";
import MockInputValidatorModule from "../../ignition/modules/mockInputValidator";

const { ignition } = await network.connect();

export async function deployInputValidatorFixture() {
  const { mockInputValidator } = await ignition.deploy(MockInputValidatorModule);
  return MockInputValidator__factory.connect(await mockInputValidator.getAddress());
}

deployInputValidatorFixture();