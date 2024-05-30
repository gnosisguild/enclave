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
    const { enclave, enclave_address, maxDuration, owner } = await this.loadFixture(deployEnclaveFixture);
    this.enclave = enclave;
    this.enclave_address = enclave_address;
    this.maxDuration = maxDuration;
    this.owner = owner;

    const { mockComputationModule, mockComputationModule_address } = await this.loadFixture(
      deployMockComputationModuleFixture,
    );
    this.mockComputationModule = mockComputationModule;
    this.mockComputationModule_address = mockComputationModule_address;
  });

  describe("Deployment", function () {
    it("correctly sets max duration", async function () {
      // We don't use the fixture here because we want a different deployment
      const maxDuration = await this.enclave.maxDuration();
      expect(maxDuration).to.equal(this.maxDuration);
    });

    it("correctly sets owner", async function () {
      // We don't use the fixture here because we want a different deployment
      const owner = await this.enclave.owner();
      expect(owner).to.equal(this.owner);
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
