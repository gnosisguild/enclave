import { anyValue } from "@nomicfoundation/hardhat-chai-matchers/withArgs";
import { loadFixture, time } from "@nomicfoundation/hardhat-network-helpers";
import { expect } from "chai";
import { ethers } from "hardhat";

import type { Signers } from "../types";
import { deployEnclaveFixture } from "./Enclave.fixture";

describe("Enclave", function () {
  before(async function () {
    this.signers = {} as Signers;

    const signers = await ethers.getSigners();
    this.signers.admin = signers[0];

    this.loadFixture = loadFixture;
  });

  describe("Deployment", function () {
    beforeEach(async function () {
      const { enclave, enclave_address, maxDuration, owner } = await this.loadFixture(deployEnclaveFixture);
      this.enclave = enclave;
      this.enclave_address = enclave_address;
      this.maxDuration = maxDuration;
      this.owner = owner;
    });

    it("Should correctly set max duration", async function () {
      // We don't use the fixture here because we want a different deployment
      const maxDuration = await this.enclave.maxDuration();
      expect(maxDuration).to.equal(this.maxDuration);
    });
  });
});
