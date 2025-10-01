// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { LeanIMT } from "@zk-kit/lean-imt";
import { expect } from "chai";
import { network } from "hardhat";
import { poseidon2 } from "poseidon-lite";

import BondingRegistryModule from "../../ignition/modules/bondingRegistry";
import EnclaveTicketTokenModule from "../../ignition/modules/enclaveTicketToken";
import EnclaveTokenModule from "../../ignition/modules/enclaveToken";
import MockCiphernodeRegistryModule from "../../ignition/modules/mockCiphernodeRegistry";
import MockStableTokenModule from "../../ignition/modules/mockStableToken";
import SlashingManagerModule from "../../ignition/modules/slashingManager";
import {
  BondingRegistry__factory as BondingRegistryFactory,
  CiphernodeRegistryOwnable__factory as CiphernodeRegistryOwnableFactory,
  EnclaveTicketToken__factory as EnclaveTicketTokenFactory,
  EnclaveToken__factory as EnclaveTokenFactory,
  MockUSDC__factory as MockUSDCFactory,
  SlashingManager__factory as SlashingManagerFactory,
} from "../../types";

const AddressOne = "0x0000000000000000000000000000000000000001";

const { ethers, networkHelpers, ignition } = await network.connect();
const { loadFixture, time } = networkHelpers;

const hash = (a: bigint, b: bigint) => poseidon2([a, b]);

const REASON_DEPOSIT = ethers.encodeBytes32String("DEPOSIT");
const REASON_WITHDRAW = ethers.encodeBytes32String("WITHDRAW");
const REASON_BOND = ethers.encodeBytes32String("BOND");
const REASON_UNBOND = ethers.encodeBytes32String("UNBOND");

describe("BondingRegistry", function () {
  const SEVEN_DAYS_IN_SECONDS = 7 * 24 * 60 * 60;
  const TICKET_PRICE = ethers.parseUnits("10", 6);
  const LICENSE_REQUIRED_BOND = ethers.parseEther("1000");
  const MIN_TICKET_BALANCE = 5;
  async function setup() {
    const [owner, operator1, operator2, treasury, notTheOwner] =
      await ethers.getSigners();

    const ownerAddress = await owner.getAddress();
    const operator1Address = await operator1.getAddress();
    const operator2Address = await operator2.getAddress();
    const treasuryAddress = await treasury.getAddress();

    const usdcContract = await ignition.deploy(MockStableTokenModule, {
      parameters: {
        MockUSDC: {
          initialSupply: 1000000,
        },
      },
    });

    const enclTokenContract = await ignition.deploy(EnclaveTokenModule, {
      parameters: {
        EnclaveToken: {
          owner: ownerAddress,
        },
      },
    });

    const ciphernodeRegistryContract = await ignition.deploy(
      MockCiphernodeRegistryModule,
      {
        parameters: {
          CiphernodeRegistry: {
            enclaveAddress: ownerAddress,
            owner: ownerAddress,
          },
        },
      },
    );

    const ticketTokenContract = await ignition.deploy(
      EnclaveTicketTokenModule,
      {
        parameters: {
          EnclaveTicketToken: {
            baseToken: await usdcContract.mockUSDC.getAddress(),
            registry: AddressOne,
            owner: ownerAddress,
          },
        },
      },
    );

    const slashingManagerContract = await ignition.deploy(
      SlashingManagerModule,
      {
        parameters: {
          SlashingManager: {
            admin: ownerAddress,
            bondingRegistry: AddressOne,
          },
        },
      },
    );

    const bondingRegistryContract = await ignition.deploy(
      BondingRegistryModule,
      {
        parameters: {
          BondingRegistry: {
            owner: ownerAddress,
            ticketToken:
              await ticketTokenContract.enclaveTicketToken.getAddress(),
            licenseToken: await enclTokenContract.enclaveToken.getAddress(),
            registry:
              await ciphernodeRegistryContract.mockCiphernodeRegistry.getAddress(),
            slashedFundsTreasury: treasuryAddress,
            ticketPrice: TICKET_PRICE,
            licenseRequiredBond: LICENSE_REQUIRED_BOND,
            minTicketBalance: MIN_TICKET_BALANCE,
            exitDelay: SEVEN_DAYS_IN_SECONDS,
          },
        },
      },
    );

    const bondingRegistry = BondingRegistryFactory.connect(
      await bondingRegistryContract.bondingRegistry.getAddress(),
      owner,
    );
    const ticketToken = EnclaveTicketTokenFactory.connect(
      await ticketTokenContract.enclaveTicketToken.getAddress(),
      owner,
    );
    const licenseToken = EnclaveTokenFactory.connect(
      await enclTokenContract.enclaveToken.getAddress(),
      owner,
    );
    const usdcToken = MockUSDCFactory.connect(
      await usdcContract.mockUSDC.getAddress(),
      owner,
    );
    const slashingManager = SlashingManagerFactory.connect(
      await slashingManagerContract.slashingManager.getAddress(),
      owner,
    );
    const ciphernodeRegistry = CiphernodeRegistryOwnableFactory.connect(
      await ciphernodeRegistryContract.mockCiphernodeRegistry.getAddress(),
      owner,
    );

    await ticketToken.setRegistry(await bondingRegistry.getAddress());
    await slashingManager.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );
    await bondingRegistry.setSlashingManager(
      await slashingManager.getAddress(),
    );

    await usdcToken.mint(ownerAddress, ethers.parseUnits("100000", 6));
    await usdcToken.mint(operator1Address, ethers.parseUnits("100000", 6));
    await usdcToken.mint(operator2Address, ethers.parseUnits("100000", 6));
    await licenseToken.mintAllocation(
      ownerAddress,
      ethers.parseEther("100000"),
      "Test allocation",
    );
    await licenseToken.mintAllocation(
      operator1Address,
      ethers.parseEther("100000"),
      "Test allocation",
    );
    await licenseToken.mintAllocation(
      operator2Address,
      ethers.parseEther("100000"),
      "Test allocation",
    );

    await licenseToken.setTransferRestriction(false);

    const tree = new LeanIMT(hash);

    return {
      bondingRegistry,
      ticketToken,
      licenseToken,
      usdcToken,
      slashingManager,
      ciphernodeRegistry,
      tree,
      owner,
      operator1,
      operator2,
      treasury,
      notTheOwner,
      ownerAddress,
      operator1Address,
      operator2Address,
      treasuryAddress,
    };
  }

  describe("constructor / initialize()", function () {
    it("correctly sets initial parameters", async function () {
      const { bondingRegistry, ticketToken, licenseToken, treasuryAddress } =
        await loadFixture(setup);

      expect(await bondingRegistry.ticketToken()).to.equal(
        await ticketToken.getAddress(),
      );
      expect(await bondingRegistry.licenseToken()).to.equal(
        await licenseToken.getAddress(),
      );
      expect(await bondingRegistry.slashedFundsTreasury()).to.equal(
        treasuryAddress,
      );
      expect(await bondingRegistry.ticketPrice()).to.equal(TICKET_PRICE);
      expect(await bondingRegistry.licenseRequiredBond()).to.equal(
        LICENSE_REQUIRED_BOND,
      );
      expect(await bondingRegistry.minTicketBalance()).to.equal(
        MIN_TICKET_BALANCE,
      );
      expect(await bondingRegistry.exitDelay()).to.equal(SEVEN_DAYS_IN_SECONDS);
      expect(await bondingRegistry.licenseActiveBps()).to.equal(8000);
    });
  });

  describe("bondLicense()", function () {
    it("allows operators to bond license tokens", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const bondAmount = ethers.parseEther("1000");
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);

      await expect(bondingRegistry.connect(operator1).bondLicense(bondAmount))
        .to.emit(bondingRegistry, "LicenseBondUpdated")
        .withArgs(
          await operator1.getAddress(),
          bondAmount,
          bondAmount,
          REASON_BOND,
        );

      expect(
        await bondingRegistry.getLicenseBond(await operator1.getAddress()),
      ).to.equal(bondAmount);
    });

    it("reverts if amount is zero", async function () {
      const { bondingRegistry, operator1 } = await loadFixture(setup);

      await expect(
        bondingRegistry.connect(operator1).bondLicense(0),
      ).to.be.revertedWithCustomError(bondingRegistry, "ZeroAmount");
    });

    it("reverts if exit is in progress", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const bondAmount = ethers.parseEther("1000");
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);

      await bondingRegistry.connect(operator1).registerOperator();

      await bondingRegistry.connect(operator1).deregisterOperator([]);

      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await expect(
        bondingRegistry.connect(operator1).bondLicense(bondAmount),
      ).to.be.revertedWithCustomError(bondingRegistry, "ExitInProgress");
    });

    it("accumulates multiple bond amounts", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const bondAmount1 = ethers.parseEther("500");
      const bondAmount2 = ethers.parseEther("300");

      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount1);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount1);

      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount2);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount2);

      expect(
        await bondingRegistry.getLicenseBond(await operator1.getAddress()),
      ).to.equal(bondAmount1 + bondAmount2);
    });
  });

  describe("unbondLicense()", function () {
    it("allows operators to unbond license tokens", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const bondAmount = ethers.parseEther("1000");
      const unbondAmount = ethers.parseEther("200");

      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);

      await expect(
        bondingRegistry.connect(operator1).unbondLicense(unbondAmount),
      )
        .to.emit(bondingRegistry, "LicenseBondUpdated")
        .withArgs(
          await operator1.getAddress(),
          -unbondAmount,
          bondAmount - unbondAmount,
          REASON_UNBOND,
        );

      expect(
        await bondingRegistry.getLicenseBond(await operator1.getAddress()),
      ).to.equal(bondAmount - unbondAmount);
    });

    it("reverts if amount is zero", async function () {
      const { bondingRegistry, operator1 } = await loadFixture(setup);

      await expect(
        bondingRegistry.connect(operator1).unbondLicense(0),
      ).to.be.revertedWithCustomError(bondingRegistry, "ZeroAmount");
    });

    it("reverts if insufficient balance", async function () {
      const { bondingRegistry, operator1 } = await loadFixture(setup);

      await expect(
        bondingRegistry
          .connect(operator1)
          .unbondLicense(ethers.parseEther("100")),
      ).to.be.revertedWithCustomError(bondingRegistry, "InsufficientBalance");
    });

    it("queues license tokens for exit", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const bondAmount = ethers.parseEther("1000");
      const unbondAmount = ethers.parseEther("200");

      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);

      await bondingRegistry.connect(operator1).unbondLicense(unbondAmount);

      const [, licensePending] = await bondingRegistry.pendingExits(
        await operator1.getAddress(),
      );
      expect(licensePending).to.equal(unbondAmount);
    });
  });

  describe("registerOperator()", function () {
    it("allows properly licensed operators to register", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);

      await bondingRegistry.connect(operator1).registerOperator();

      expect(await bondingRegistry.isRegistered(await operator1.getAddress()))
        .to.be.true;
      expect(await bondingRegistry.isActive(await operator1.getAddress())).to.be
        .false;
    });

    it("reverts if not properly licensed", async function () {
      const { bondingRegistry, operator1 } = await loadFixture(setup);

      await expect(
        bondingRegistry.connect(operator1).registerOperator(),
      ).to.be.revertedWithCustomError(bondingRegistry, "NotLicensed");
    });

    it("reverts if already registered", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      await expect(
        bondingRegistry.connect(operator1).registerOperator(),
      ).to.be.revertedWithCustomError(bondingRegistry, "AlreadyRegistered");
    });

    it("clears previous exit request when re-registering", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      await bondingRegistry.connect(operator1).deregisterOperator([]);

      await time.increase(SEVEN_DAYS_IN_SECONDS + 1);

      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      expect(
        await bondingRegistry.hasExitInProgress(await operator1.getAddress()),
      ).to.be.false;
    });
  });

  describe("deregisterOperator()", function () {
    it("allows registered operators to deregister", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      const latestTime = await time.latest();
      await expect(bondingRegistry.connect(operator1).deregisterOperator([]))
        .to.emit(bondingRegistry, "CiphernodeDeregistrationRequested")
        .withArgs(
          await operator1.getAddress(),
          latestTime + SEVEN_DAYS_IN_SECONDS + 1,
        );

      expect(await bondingRegistry.isRegistered(await operator1.getAddress()))
        .to.be.false;
      expect(
        await bondingRegistry.hasExitInProgress(await operator1.getAddress()),
      ).to.be.true;
    });

    it("reverts if not registered", async function () {
      const { bondingRegistry, operator1 } = await loadFixture(setup);

      await expect(
        bondingRegistry.connect(operator1).deregisterOperator([]),
      ).to.be.revertedWithCustomError(bondingRegistry, "NotRegistered");
    });

    it("queues assets for exit when deregistering", async function () {
      const {
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        operator1,
      } = await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      const ticketAmount = ethers.parseUnits("100", 6);
      await usdcToken
        .connect(operator1)
        .approve(await ticketToken.getAddress(), ticketAmount);
      await bondingRegistry.connect(operator1).addTicketBalance(ticketAmount);

      await bondingRegistry.connect(operator1).deregisterOperator([]);

      const [ticketPending, licensePending] =
        await bondingRegistry.pendingExits(await operator1.getAddress());
      expect(ticketPending).to.equal(ticketAmount);
      expect(licensePending).to.equal(bondAmount);
    });
  });

  describe("addTicketBalance()", function () {
    it("allows registered operators to add ticket balance", async function () {
      const {
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        operator1,
      } = await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      const ticketAmount = ethers.parseUnits("100", 6);
      await usdcToken
        .connect(operator1)
        .approve(await ticketToken.getAddress(), ticketAmount);

      await expect(
        bondingRegistry.connect(operator1).addTicketBalance(ticketAmount),
      )
        .to.emit(bondingRegistry, "TicketBalanceUpdated")
        .withArgs(
          await operator1.getAddress(),
          ticketAmount,
          ticketAmount,
          REASON_DEPOSIT,
        );

      expect(
        await bondingRegistry.getTicketBalance(await operator1.getAddress()),
      ).to.equal(ticketAmount);
    });

    it("activates operator when minimum balance is reached", async function () {
      const {
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        operator1,
      } = await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      const ticketAmount = ethers.parseUnits("50", 6);
      await usdcToken
        .connect(operator1)
        .approve(await ticketToken.getAddress(), ticketAmount);

      await expect(
        bondingRegistry.connect(operator1).addTicketBalance(ticketAmount),
      )
        .to.emit(bondingRegistry, "OperatorActivationChanged")
        .withArgs(await operator1.getAddress(), true);

      expect(await bondingRegistry.isActive(await operator1.getAddress())).to.be
        .true;
    });

    it("reverts if not registered", async function () {
      const { bondingRegistry, operator1 } = await loadFixture(setup);

      await expect(
        bondingRegistry
          .connect(operator1)
          .addTicketBalance(ethers.parseUnits("100", 6)),
      ).to.be.revertedWithCustomError(bondingRegistry, "NotRegistered");
    });

    it("reverts if amount is zero", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      await expect(
        bondingRegistry.connect(operator1).addTicketBalance(0),
      ).to.be.revertedWithCustomError(bondingRegistry, "ZeroAmount");
    });
  });

  describe("removeTicketBalance()", function () {
    it("allows operators to remove ticket balance", async function () {
      const {
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        operator1,
      } = await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      const ticketAmount = ethers.parseUnits("100", 6);
      await usdcToken
        .connect(operator1)
        .approve(await ticketToken.getAddress(), ticketAmount);
      await bondingRegistry.connect(operator1).addTicketBalance(ticketAmount);

      const removeAmount = ethers.parseUnits("30", 6);
      await expect(
        bondingRegistry.connect(operator1).removeTicketBalance(removeAmount),
      )
        .to.emit(bondingRegistry, "TicketBalanceUpdated")
        .withArgs(
          await operator1.getAddress(),
          -removeAmount,
          ticketAmount - removeAmount,
          REASON_WITHDRAW,
        );

      expect(
        await bondingRegistry.getTicketBalance(await operator1.getAddress()),
      ).to.equal(ticketAmount - removeAmount);
    });

    it("queues removed tickets for exit", async function () {
      const {
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        operator1,
      } = await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      const ticketAmount = ethers.parseUnits("100", 6);
      await usdcToken
        .connect(operator1)
        .approve(await ticketToken.getAddress(), ticketAmount);
      await bondingRegistry.connect(operator1).addTicketBalance(ticketAmount);

      const removeAmount = ethers.parseUnits("30", 6);
      await bondingRegistry
        .connect(operator1)
        .removeTicketBalance(removeAmount);

      const [ticketPending] = await bondingRegistry.pendingExits(
        await operator1.getAddress(),
      );
      expect(ticketPending).to.equal(removeAmount);
    });

    it("deactivates operator if balance falls below minimum", async function () {
      const {
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        operator1,
      } = await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      const ticketAmount = ethers.parseUnits("60", 6);
      await usdcToken
        .connect(operator1)
        .approve(await ticketToken.getAddress(), ticketAmount);
      await bondingRegistry.connect(operator1).addTicketBalance(ticketAmount);

      const removeAmount = ethers.parseUnits("20", 6);
      await expect(
        bondingRegistry.connect(operator1).removeTicketBalance(removeAmount),
      )
        .to.emit(bondingRegistry, "OperatorActivationChanged")
        .withArgs(await operator1.getAddress(), false);

      expect(await bondingRegistry.isActive(await operator1.getAddress())).to.be
        .false;
    });

    it("reverts if insufficient balance", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      await expect(
        bondingRegistry
          .connect(operator1)
          .removeTicketBalance(ethers.parseUnits("100", 6)),
      ).to.be.revertedWithCustomError(bondingRegistry, "InsufficientBalance");
    });
  });

  describe("claimExits()", function () {
    it("allows claiming after exit delay", async function () {
      const {
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        operator1,
      } = await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      const ticketAmount = ethers.parseUnits("100", 6);
      await usdcToken
        .connect(operator1)
        .approve(await ticketToken.getAddress(), ticketAmount);
      await bondingRegistry.connect(operator1).addTicketBalance(ticketAmount);

      await bondingRegistry.connect(operator1).deregisterOperator([]);

      await time.increase(SEVEN_DAYS_IN_SECONDS + 1);

      const initialUSDCBalance = await usdcToken.balanceOf(
        await operator1.getAddress(),
      );
      const initialENCLBalance = await licenseToken.balanceOf(
        await operator1.getAddress(),
      );

      await bondingRegistry
        .connect(operator1)
        .claimExits(ticketAmount, bondAmount);

      expect(await usdcToken.balanceOf(await operator1.getAddress())).to.equal(
        initialUSDCBalance + ticketAmount,
      );
      expect(
        await licenseToken.balanceOf(await operator1.getAddress()),
      ).to.equal(initialENCLBalance + bondAmount);
    });

    it("reverts if exit not ready", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      await bondingRegistry.connect(operator1).deregisterOperator([]);

      await expect(
        bondingRegistry.connect(operator1).claimExits(0, bondAmount),
      ).to.be.revertedWithCustomError(bondingRegistry, "ExitNotReady");
    });

    it("allows partial claims", async function () {
      const {
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        operator1,
      } = await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      const ticketAmount = ethers.parseUnits("100", 6);
      await usdcToken
        .connect(operator1)
        .approve(await ticketToken.getAddress(), ticketAmount);
      await bondingRegistry.connect(operator1).addTicketBalance(ticketAmount);

      await bondingRegistry.connect(operator1).deregisterOperator([]);

      await time.increase(SEVEN_DAYS_IN_SECONDS + 1);

      const partialTickets = ethers.parseUnits("50", 6);
      const partialLicense = ethers.parseEther("500");

      const initialUSDCBalance = await usdcToken.balanceOf(
        await operator1.getAddress(),
      );
      const initialENCLBalance = await licenseToken.balanceOf(
        await operator1.getAddress(),
      );

      await bondingRegistry
        .connect(operator1)
        .claimExits(partialTickets, partialLicense);

      expect(await usdcToken.balanceOf(await operator1.getAddress())).to.equal(
        initialUSDCBalance + partialTickets,
      );
      expect(
        await licenseToken.balanceOf(await operator1.getAddress()),
      ).to.equal(initialENCLBalance + partialLicense);

      const [remainingTickets, remainingLicense] =
        await bondingRegistry.pendingExits(await operator1.getAddress());
      expect(remainingTickets).to.equal(ticketAmount - partialTickets);
      expect(remainingLicense).to.equal(bondAmount - partialLicense);
    });
  });

  describe("isLicensed()", function () {
    it("returns true when operator has minimum license bond", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const minBond = (LICENSE_REQUIRED_BOND * 8000n) / 10000n;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), minBond);
      await bondingRegistry.connect(operator1).bondLicense(minBond);

      expect(await bondingRegistry.isLicensed(await operator1.getAddress())).to
        .be.true;
    });

    it("returns false when operator has insufficient license bond", async function () {
      const { bondingRegistry, licenseToken, operator1 } =
        await loadFixture(setup);

      const insufficientBond = (LICENSE_REQUIRED_BOND * 7999n) / 10000n;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), insufficientBond);
      await bondingRegistry.connect(operator1).bondLicense(insufficientBond);

      expect(await bondingRegistry.isLicensed(await operator1.getAddress())).to
        .be.false;
    });
  });

  describe("availableTickets()", function () {
    it("calculates available tickets correctly", async function () {
      const {
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        operator1,
      } = await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      const ticketAmount = ethers.parseUnits("100", 6);
      await usdcToken
        .connect(operator1)
        .approve(await ticketToken.getAddress(), ticketAmount);
      await bondingRegistry.connect(operator1).addTicketBalance(ticketAmount);

      expect(
        await bondingRegistry.availableTickets(await operator1.getAddress()),
      ).to.equal(10);
    });

    it("returns 0 when operator has zero ticket balance", async function () {
      const { bondingRegistry, operator1 } = await loadFixture(setup);

      expect(
        await bondingRegistry.availableTickets(await operator1.getAddress()),
      ).to.equal(0);
    });
  });

  describe("Admin Functions", function () {
    describe("setTicketPrice()", function () {
      it("allows owner to set ticket price", async function () {
        const { bondingRegistry } = await loadFixture(setup);

        const newPrice = ethers.parseUnits("15", 6);
        await expect(bondingRegistry.setTicketPrice(newPrice))
          .to.emit(bondingRegistry, "ConfigurationUpdated")
          .withArgs(
            ethers.encodeBytes32String("ticketPrice"),
            TICKET_PRICE,
            newPrice,
          );

        expect(await bondingRegistry.ticketPrice()).to.equal(newPrice);
      });

      it("reverts if price is zero", async function () {
        const { bondingRegistry } = await loadFixture(setup);

        await expect(
          bondingRegistry.setTicketPrice(0),
        ).to.be.revertedWithCustomError(
          bondingRegistry,
          "InvalidConfiguration",
        );
      });

      it("reverts if not owner", async function () {
        const { bondingRegistry, notTheOwner } = await loadFixture(setup);

        await expect(
          bondingRegistry
            .connect(notTheOwner)
            .setTicketPrice(ethers.parseEther("15")),
        ).to.be.revertedWithCustomError(
          bondingRegistry,
          "OwnableUnauthorizedAccount",
        );
      });
    });

    describe("setLicenseActiveBps()", function () {
      it("allows owner to set license active basis points", async function () {
        const { bondingRegistry } = await loadFixture(setup);

        const newBps = 9000;
        await expect(bondingRegistry.setLicenseActiveBps(newBps))
          .to.emit(bondingRegistry, "ConfigurationUpdated")
          .withArgs(
            ethers.encodeBytes32String("licenseActiveBps"),
            8000,
            newBps,
          );

        expect(await bondingRegistry.licenseActiveBps()).to.equal(newBps);
      });

      it("reverts if bps is 0", async function () {
        const { bondingRegistry } = await loadFixture(setup);

        await expect(
          bondingRegistry.setLicenseActiveBps(0),
        ).to.be.revertedWithCustomError(
          bondingRegistry,
          "InvalidConfiguration",
        );
      });

      it("reverts if bps is greater than 10000", async function () {
        const { bondingRegistry } = await loadFixture(setup);

        await expect(
          bondingRegistry.setLicenseActiveBps(10001),
        ).to.be.revertedWithCustomError(
          bondingRegistry,
          "InvalidConfiguration",
        );
      });
    });

    describe("withdrawSlashedFunds()", function () {
      it("allows owner to withdraw slashed funds", async function () {
        const { bondingRegistry, treasury } = await loadFixture(setup);

        await expect(bondingRegistry.withdrawSlashedFunds(0, 0))
          .to.emit(bondingRegistry, "SlashedFundsWithdrawn")
          .withArgs(await treasury.getAddress(), 0, 0);
      });

      it("reverts if not owner", async function () {
        const { bondingRegistry, notTheOwner } = await loadFixture(setup);

        await expect(
          bondingRegistry.connect(notTheOwner).withdrawSlashedFunds(0, 0),
        ).to.be.revertedWithCustomError(
          bondingRegistry,
          "OwnableUnauthorizedAccount",
        );
      });
    });
  });

  describe("Edge Cases and Complex Scenarios", function () {
    it("handles operator becoming inactive due to license reduction", async function () {
      const {
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        operator1,
      } = await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      const ticketAmount = ethers.parseUnits("60", 6);
      await usdcToken
        .connect(operator1)
        .approve(await ticketToken.getAddress(), ticketAmount);
      await bondingRegistry.connect(operator1).addTicketBalance(ticketAmount);

      expect(await bondingRegistry.isActive(await operator1.getAddress())).to.be
        .true;

      const unbondAmount = LICENSE_REQUIRED_BOND / 5n;
      await bondingRegistry.connect(operator1).unbondLicense(unbondAmount + 1n);
      expect(await bondingRegistry.isActive(await operator1.getAddress())).to.be
        .false;
      expect(await bondingRegistry.isLicensed(await operator1.getAddress())).to
        .be.false;
    });

    it("handles multiple operators with different states", async function () {
      const {
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        operator1,
        operator2,
      } = await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();

      await licenseToken
        .connect(operator2)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator2).bondLicense(bondAmount);
      await bondingRegistry.connect(operator2).registerOperator();

      const ticketAmount = ethers.parseUnits("60", 6);
      await usdcToken
        .connect(operator2)
        .approve(await ticketToken.getAddress(), ticketAmount);
      await bondingRegistry.connect(operator2).addTicketBalance(ticketAmount);

      expect(await bondingRegistry.isRegistered(await operator1.getAddress()))
        .to.be.true;
      expect(await bondingRegistry.isActive(await operator1.getAddress())).to.be
        .false;

      expect(await bondingRegistry.isRegistered(await operator2.getAddress()))
        .to.be.true;
      expect(await bondingRegistry.isActive(await operator2.getAddress())).to.be
        .true;
    });

    it("handles the complete operator lifecycle", async function () {
      const {
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        operator1,
      } = await loadFixture(setup);

      const bondAmount = LICENSE_REQUIRED_BOND;
      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      expect(await bondingRegistry.isLicensed(await operator1.getAddress())).to
        .be.true;

      await bondingRegistry.connect(operator1).registerOperator();
      expect(await bondingRegistry.isRegistered(await operator1.getAddress()))
        .to.be.true;
      expect(await bondingRegistry.isActive(await operator1.getAddress())).to.be
        .false;

      const ticketAmount = ethers.parseUnits("60", 6);
      await usdcToken
        .connect(operator1)
        .approve(await ticketToken.getAddress(), ticketAmount);
      await bondingRegistry.connect(operator1).addTicketBalance(ticketAmount);
      expect(await bondingRegistry.isActive(await operator1.getAddress())).to.be
        .true;

      await bondingRegistry.connect(operator1).deregisterOperator([]);
      expect(await bondingRegistry.isRegistered(await operator1.getAddress()))
        .to.be.false;
      expect(
        await bondingRegistry.hasExitInProgress(await operator1.getAddress()),
      ).to.be.true;

      await time.increase(SEVEN_DAYS_IN_SECONDS + 1);

      const initialUSDCBalance = await usdcToken.balanceOf(
        await operator1.getAddress(),
      );
      const initialENCLBalance = await licenseToken.balanceOf(
        await operator1.getAddress(),
      );

      await bondingRegistry
        .connect(operator1)
        .claimExits(ticketAmount, bondAmount);

      expect(await usdcToken.balanceOf(await operator1.getAddress())).to.equal(
        initialUSDCBalance + ticketAmount,
      );
      expect(
        await licenseToken.balanceOf(await operator1.getAddress()),
      ).to.equal(initialENCLBalance + bondAmount);

      await licenseToken
        .connect(operator1)
        .approve(await bondingRegistry.getAddress(), bondAmount);
      await bondingRegistry.connect(operator1).bondLicense(bondAmount);
      await bondingRegistry.connect(operator1).registerOperator();
      expect(await bondingRegistry.isRegistered(await operator1.getAddress()))
        .to.be.true;
    });
  });
});
