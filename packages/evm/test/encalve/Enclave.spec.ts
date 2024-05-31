import { anyValue } from "@nomicfoundation/hardhat-chai-matchers/withArgs";
import { loadFixture, time } from "@nomicfoundation/hardhat-network-helpers";
import { expect } from "chai";
import { ethers } from "hardhat";

import { deployMockComputationModuleFixture } from "../mocks/MockComputationModule.fixture";
import type { Signers } from "../types";
import { deployEnclaveFixture } from "./Enclave.fixture";

describe("Enclave", function () {
  before(async function () {
    this.signers = {} as Signers;

    const signers = await ethers.getSigners();
    this.signers.admin = signers[0];

    this.loadFixture = loadFixture;
  });

  beforeEach(async function () {
    const { Enclave, enclave, enclave_address, maxDuration, owner, otherAccount } =
      await this.loadFixture(deployEnclaveFixture);
    this.Enclave = Enclave;
    this.enclave = enclave;
    this.enclave_address = enclave_address;
    this.maxDuration = maxDuration;
    this.owner = owner;
    this.otherAccount = otherAccount;

    const { mockComputationModule, mockComputationModule_address } = await this.loadFixture(
      deployMockComputationModuleFixture,
    );
    this.mockComputationModule = mockComputationModule;
    this.mockComputationModule_address = mockComputationModule_address;
  });

  describe("Deployment", function () {
    it("correctly sets max duration", async function () {
      const maxDuration = await this.enclave.maxDuration();

      expect(maxDuration).to.equal(this.maxDuration);
    });

    it("correctly sets owner", async function () {
      // uses the fixture deployment with this.owner set as the owner
      const owner1 = await this.enclave.owner();
      // create a new deployment with this.otherAccount as the owner
      // note that this.owner is msg.sender in both cases
      const enclave = await this.Enclave.deploy(this.otherAccount.address, this.maxDuration);
      const owner2 = await enclave.owner();

      // expect the owner to be the same as the one set in the fixture
      expect(owner1).to.equal(this.owner);
      // expect the owner to be the same as the one set in the new deployment
      expect(owner2).to.equal(this.otherAccount);
    });
  });

  describe("setMaxDuration()", function () {
    it("reverts if not called by owner");
    it("reverts if duration is 0");
    it("reverts if duration is greater than 30 days");
    it("set max duration correctly");
    it("returns true if max duration is set successfully");
    it("emits MaxDurationChanged event");
  });

  describe("getE3Id()", function () {
    it("reverts if E3 does not exist");
    it("returns correct E3 details");
  });
});
