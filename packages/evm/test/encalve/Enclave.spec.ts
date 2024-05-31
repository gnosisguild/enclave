import { anyValue } from "@nomicfoundation/hardhat-chai-matchers/withArgs";
import { loadFixture, time } from "@nomicfoundation/hardhat-network-helpers";
import { expect } from "chai";
import { ethers } from "hardhat";

import func from "../../deploy/deploy";
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

  describe("constructor / initialize()", function () {
    it("correctly sets owner", async function () {
      // uses the fixture deployment with this.owner set as the owner
      const owner1 = await this.enclave.owner();
      // create a new deployment with this.otherAccount as the owner
      // note that this.owner is msg.sender in both cases
      const enclave = await this.Enclave.deploy(this.otherAccount.address, this.otherAccount.address, this.maxDuration);
      const owner2 = await enclave.owner();

      // expect the owner to be the same as the one set in the fixture
      expect(owner1).to.equal(this.owner);
      // expect the owner to be the same as the one set in the new deployment
      expect(owner2).to.equal(this.otherAccount);
    });

    it("correctly sets cypherNodeRegistry address", async function () {
      const cypherNodeRegistry = await this.enclave.cypherNodeRegistry();

      expect(cypherNodeRegistry).to.equal(this.otherAccount.address);
    });

    it("correctly sets max duration", async function () {
      const maxDuration = await this.enclave.maxDuration();

      expect(maxDuration).to.equal(this.maxDuration);
    });
  });

  describe("setMaxDuration()", function () {
    it("reverts if not called by owner", async function () {
      await expect(this.enclave.connect(this.otherAccount).setMaxDuration(1))
        .to.be.revertedWithCustomError(this.enclave, "OwnableUnauthorizedAccount")
        .withArgs(this.otherAccount.address);
    });
    it("set max duration correctly", async function () {
      await this.enclave.setMaxDuration(1);

      const maxDuration = await this.enclave.maxDuration();
      expect(maxDuration).to.equal(1);
    });
    it("returns true if max duration is set successfully", async function () {
      const result = await this.enclave.setMaxDuration.staticCall(1);

      expect(result).to.be.true;
    });
    it("emits MaxDurationSet event", async function () {
      await expect(this.enclave.setMaxDuration(1)).to.emit(this.enclave, "MaxDurationSet").withArgs(1);
    });
  });

  describe("setCypherNodeRegistry()", function () {
    it("reverts if not called by owner", async function () {
      await expect(this.enclave.connect(this.otherAccount).setCypherNodeRegistry(this.otherAccount.address))
        .to.be.revertedWithCustomError(this.enclave, "OwnableUnauthorizedAccount")
        .withArgs(this.otherAccount.address);
    });
    it("reverts if given address(0)", async function () {
      await expect(this.enclave.setCypherNodeRegistry(ethers.ZeroAddress))
        .to.be.revertedWithCustomError(this.enclave, "InvalidCypherNodeRegistry")
        .withArgs(ethers.ZeroAddress);
    });
    it("reverts if given address is the same as the current cypherNodeRegistry", async function () {
      await expect(this.enclave.setCypherNodeRegistry(this.otherAccount.address))
        .to.be.revertedWithCustomError(this.enclave, "InvalidCypherNodeRegistry")
        .withArgs(this.otherAccount.address);
    });
    it("sets cypherNodeRegistry correctly", async function () {
      await this.enclave.setCypherNodeRegistry(this.owner.address);

      const cypherNodeRegistry = await this.enclave.cypherNodeRegistry();
      expect(cypherNodeRegistry).to.equal(this.owner.address);
    });
    it("returns true if cypherNodeRegistry is set successfully", async function () {
      const result = await this.enclave.setCypherNodeRegistry.staticCall(this.owner.address);

      expect(result).to.be.true;
    });
    it("emits CypherNodeRegistrySet event", async function () {
      await expect(this.enclave.setCypherNodeRegistry(this.owner.address))
        .to.emit(this.enclave, "CypherNodeRegistrySet")
        .withArgs(this.owner.address);
    });
  });

  describe("getE3()", function () {
    it("reverts if E3 does not exist", async function () {
      await expect(this.enclave.getE3(1)).to.be.revertedWithCustomError(this.enclave, "E3DoesNotExist").withArgs(1);
    });
    it("returns correct E3 details");
  });

  describe("enableComputationModule()", function () {
    it("reverts if not called by owner", async function () {
      await expect(this.enclave.connect(this.otherAccount).enableComputationModule(this.mockComputationModule_address))
        .to.be.revertedWithCustomError(this.enclave, "OwnableUnauthorizedAccount")
        .withArgs(this.otherAccount);
    });
    it("reverts if computation module is already enabled", async function () {
      await this.enclave.enableComputationModule(this.mockComputationModule_address);

      await expect(this.enclave.enableComputationModule(this.mockComputationModule_address))
        .to.be.revertedWithCustomError(this.enclave, "ModuleAlreadyEnabled")
        .withArgs(this.mockComputationModule_address);
    });
    it("enables computation module correctly", async function () {
      await this.enclave.enableComputationModule(this.mockComputationModule_address);

      const enabled = await this.enclave.computationModules(this.mockComputationModule_address);
      expect(enabled).to.be.true;
    });
    it("returns true if computation module is enabled successfully", async function () {
      const result = await this.enclave.enableComputationModule.staticCall(this.mockComputationModule_address);

      expect(result).to.be.true;
    });
    it("emits ComputationModuleEnabled event", async function () {
      await expect(this.enclave.enableComputationModule(this.mockComputationModule_address))
        .to.emit(this.enclave, "ComputationModuleEnabled")
        .withArgs(this.mockComputationModule_address);
    });
  });

  describe("disableComputationModule()", function () {
    it("reverts if not called by owner", async function () {
      await expect(this.enclave.connect(this.otherAccount).disableComputationModule(this.otherAccount.address))
        .to.be.revertedWithCustomError(this.enclave, "OwnableUnauthorizedAccount")
        .withArgs(this.otherAccount.address);
    });
    it("reverts if computation module is not enabled", async function () {
      await expect(this.enclave.disableComputationModule(this.otherAccount.address))
        .to.be.revertedWithCustomError(this.enclave, "ModuleNotEnabled")
        .withArgs(this.otherAccount.address);
    });
    it("disables computation module correctly", async function () {
      await this.enclave.enableComputationModule(this.otherAccount.address);
      await this.enclave.disableComputationModule(this.otherAccount.address);

      const enabled = await this.enclave.computationModules(this.otherAccount.address);
      expect(enabled).to.be.false;
    });
    it("returns true if computation module is disabled successfully", async function () {
      await this.enclave.enableComputationModule(this.otherAccount.address);
      const result = await this.enclave.disableComputationModule.staticCall(this.otherAccount.address);

      expect(result).to.be.true;
    });
    it("emits ComputationModuleDisabled event", async function () {
      await this.enclave.enableComputationModule(this.otherAccount.address);
      await expect(this.enclave.disableComputationModule(this.otherAccount.address))
        .to.emit(this.enclave, "ComputationModuleDisabled")
        .withArgs(this.otherAccount.address);
    });
  });

  describe("enableExecutionModule()", function () {
    it("reverts if not called by owner", async function () {
      await expect(this.enclave.connect(this.otherAccount).enableExecutionModule(this.otherAccount.address))
        .to.be.revertedWithCustomError(this.enclave, "OwnableUnauthorizedAccount")
        .withArgs(this.otherAccount.address);
    });
    it("reverts if execution module is already enabled", async function () {
      await this.enclave.enableExecutionModule(this.otherAccount.address);

      await expect(this.enclave.enableExecutionModule(this.otherAccount.address))
        .to.be.revertedWithCustomError(this.enclave, "ModuleAlreadyEnabled")
        .withArgs(this.otherAccount.address);
    });
    it("enables execution module correctly", async function () {
      await this.enclave.enableExecutionModule(this.otherAccount.address);

      const enabled = await this.enclave.executionModules(this.otherAccount.address);
      expect(enabled).to.be.true;
    });
    it("returns true if execution module is enabled successfully", async function () {
      const result = await this.enclave.enableExecutionModule.staticCall(this.otherAccount.address);

      expect(result).to.be.true;
    });
    it("emits ExecutionModuleEnabled event", async function () {
      await expect(this.enclave.enableExecutionModule(this.otherAccount.address))
        .to.emit(this.enclave, "ExecutionModuleEnabled")
        .withArgs(this.otherAccount.address);
    });
  });

  describe("disableExecutionModule()", function () {
    it("reverts if not called by owner", async function () {
      await expect(this.enclave.connect(this.otherAccount).disableExecutionModule(this.otherAccount.address))
        .to.be.revertedWithCustomError(this.enclave, "OwnableUnauthorizedAccount")
        .withArgs(this.otherAccount.address);
    });
    it("reverts if execution module is not enabled", async function () {
      await expect(this.enclave.disableExecutionModule(this.otherAccount.address))
        .to.be.revertedWithCustomError(this.enclave, "ModuleNotEnabled")
        .withArgs(this.otherAccount.address);
    });
    it("disables execution module correctly", async function () {
      await this.enclave.enableExecutionModule(this.otherAccount.address);
      await this.enclave.disableExecutionModule(this.otherAccount.address);

      const enabled = await this.enclave.executionModules(this.otherAccount.address);
      expect(enabled).to.be.false;
    });
    it("returns true if execution module is disabled successfully", async function () {
      await this.enclave.enableExecutionModule(this.otherAccount.address);
      const result = await this.enclave.disableExecutionModule.staticCall(this.otherAccount.address);

      expect(result).to.be.true;
    });
    it("emits ExecutionModuleDisabled event", async function () {
      await this.enclave.enableExecutionModule(this.otherAccount.address);
      await expect(this.enclave.disableExecutionModule(this.otherAccount.address))
        .to.emit(this.enclave, "ExecutionModuleDisabled")
        .withArgs(this.otherAccount.address);
    });
  });
});
