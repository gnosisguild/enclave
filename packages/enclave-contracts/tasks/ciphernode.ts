// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { LeanIMT } from "@zk-kit/lean-imt";
import { ZeroAddress } from "ethers";
import { task } from "hardhat/config";
import { poseidon2 } from "poseidon-lite";

export const ciphernodeAdd = task(
  "ciphernode:add",
  "Register a ciphernode to the bonding registry and ciphernode registry",
)
  .addOption({
    name: "licenseBondAmount",
    description:
      "amount of ENCL to bond (in wei, e.g., 1000000000000000000000 for 1000 ENCL)",
    defaultValue: "1000000000000000000000",
  })
  .addOption({
    name: "ticketAmount",
    description:
      "amount of USDC to deposit for tickets (in wei, e.g., 1,000,000,000 for 1000 USDC)",
    defaultValue: "1000000000",
  })
  .setAction(async () => ({
    default: async ({ licenseBondAmount, ticketAmount }, hre) => {
      const connection = await hre.network.connect();
      const { ethers } = connection;

      const [signer] = await ethers.getSigners();
      console.log(`Registering ciphernode: ${signer.address}`);

      const { deployAndSaveBondingRegistry } = await import(
        "../scripts/deployAndSave/bondingRegistry"
      );
      const { deployAndSaveEnclaveTicketToken } = await import(
        "../scripts/deployAndSave/enclaveTicketToken"
      );
      const { deployAndSaveEnclaveToken } = await import(
        "../scripts/deployAndSave/enclaveToken"
      );
      const { deployAndSaveMockStableToken } = await import(
        "../scripts/deployAndSave/mockStableToken"
      );
      const { bondingRegistry } = await deployAndSaveBondingRegistry({ hre });
      const { enclaveToken } = await deployAndSaveEnclaveToken({ hre });
      const { enclaveTicketToken } = await deployAndSaveEnclaveTicketToken({
        hre,
      });
      const { mockStableToken } = await deployAndSaveMockStableToken({ hre });

      const licenseToken = enclaveToken.connect(signer);
      const ticketToken = enclaveTicketToken.connect(signer);
      const usdcToken = mockStableToken.connect(signer);
      const bondingRegistryConnected = bondingRegistry.connect(signer);

      try {
        console.log("Step 1: Checking balances...");
        const enclBalance = await licenseToken.balanceOf(signer.address);
        const usdcBalance = await usdcToken.balanceOf(signer.address);

        console.log(`ENCL balance: ${ethers.formatEther(enclBalance)}`);
        console.log(`USDC balance: ${ethers.formatUnits(usdcBalance, 6)}`);

        const licenseBondAmountBigInt = BigInt(licenseBondAmount);
        const ticketAmountBigInt = BigInt(ticketAmount);

        if (enclBalance < licenseBondAmountBigInt) {
          throw new Error(
            `Insufficient ENCL balance. Need: ${ethers.formatEther(licenseBondAmountBigInt)}, Have: ${ethers.formatEther(enclBalance)}`,
          );
        }

        if (usdcBalance < ticketAmountBigInt) {
          throw new Error(
            `Insufficient USDC balance. Need: ${ethers.formatUnits(ticketAmountBigInt, 6)}, Have: ${ethers.formatUnits(usdcBalance, 6)}`,
          );
        }

        console.log("Step 2: Approving ENCL for license bond...");
        const approveTx = await licenseToken.approve(
          await bondingRegistry.getAddress(),
          licenseBondAmountBigInt,
        );
        await approveTx.wait();
        console.log("ENCL approved");

        console.log("Step 3: Bonding license...");
        const bondTx = await bondingRegistryConnected.bondLicense(
          licenseBondAmountBigInt,
        );
        await bondTx.wait();
        console.log(
          `Licensed bonded: ${ethers.formatEther(licenseBondAmountBigInt)} ENCL`,
        );

        console.log("Step 4: Registering as operator...");
        const isRegistered = await bondingRegistry.isRegistered(signer.address);
        if (!isRegistered) {
          const registerTx = await bondingRegistryConnected.registerOperator();
          await registerTx.wait();
          console.log(
            "Operator registered (automatically added to CiphernodeRegistry)",
          );
        } else {
          console.log("Ciphernode is already registered as operator");
        }

        console.log("Step 5: Approving USDC for ticket purchase...");
        const approveUsdcTx = await usdcToken.approve(
          ticketToken.getAddress(),
          ticketAmountBigInt,
        );
        await approveUsdcTx.wait();
        console.log("USDC approved");

        console.log("Step 6: Adding ticket balance...");
        const ticketTx =
          await bondingRegistryConnected.addTicketBalance(ticketAmountBigInt);
        await ticketTx.wait();
        console.log(
          `Ticket balance added: ${ethers.formatUnits(ticketAmountBigInt, 6)} USDC worth`,
        );

        const isActive = await bondingRegistry.isActive(signer.address);
        const licenseBond = await bondingRegistry.getLicenseBond(
          signer.address,
        );
        const ticketBalance = await bondingRegistry.getTicketBalance(
          signer.address,
        );

        console.log("\n=== Registration Complete ===");
        console.log(`Ciphernode: ${signer.address}`);
        console.log(`Registered: ${isRegistered}`);
        console.log(`Active: ${isActive}`);
        console.log(`License Bond: ${ethers.formatEther(licenseBond)} ENCL`);
        console.log(
          `Ticket Balance: ${ethers.formatUnits(ticketBalance, 6)} USDC worth`,
        );
      } catch (error) {
        console.error("Registration failed:", error);
        throw error;
      }
    },
  }))
  .build();

export const ciphernodeRemove = task(
  "ciphernode:remove",
  "Deregister a ciphernode from the bonding registry",
)
  .addOption({
    name: "siblings",
    description: "comma separated siblings from tree proof",
    defaultValue: "",
  })
  .setAction(async () => ({
    default: async ({ siblings }, hre) => {
      const connection = await hre.network.connect();
      const { ethers } = connection;

      const [signer] = await ethers.getSigners();
      console.log(`Deregistering ciphernode: ${signer.address}`);

      const { deployAndSaveBondingRegistry } = await import(
        "../scripts/deployAndSave/bondingRegistry"
      );
      const { bondingRegistry } = await deployAndSaveBondingRegistry({ hre });

      const bondingRegistryConnected = bondingRegistry.connect(signer);

      const siblingsArray = siblings.split(",").map((s: string) => BigInt(s));

      try {
        console.log(
          "Deregistering operator (will also remove from CiphernodeRegistry)...",
        );
        const tx =
          await bondingRegistryConnected.deregisterOperator(siblingsArray);
        await tx.wait();

        console.log(`Ciphernode ${signer.address} deregistered`);
        console.log(
          "Note: Funds are now in exit queue. Use claimExits() after the exit delay period.",
        );
      } catch (error) {
        console.error("Deregistration failed:", error);
        throw error;
      }
    },
  }))
  .build();

export const ciphernodeMintTokens = task(
  "ciphernode:mint-tokens",
  "Mint ENCL and USDC tokens for a ciphernode (for testing)",
)
  .addOption({
    name: "ciphernodeAddress",
    description: "address of ciphernode to mint tokens for",
    defaultValue: ZeroAddress,
  })
  .addOption({
    name: "enclAmount",
    description:
      "amount of ENCL to mint (in ether units, e.g., 2000 for 2000 ENCL)",
    defaultValue: "2000",
  })
  .addOption({
    name: "usdcAmount",
    description:
      "amount of USDC to mint (in USDC units, e.g., 1000 for 1000 USDC)",
    defaultValue: "1000",
  })
  .setAction(async () => ({
    default: async ({ ciphernodeAddress, enclAmount, usdcAmount }, hre) => {
      const connection = await hre.network.connect();
      const { ethers } = connection;

      if (ciphernodeAddress === ZeroAddress) {
        throw new Error(
          "Ciphernode address is required. Use --ciphernode-address option.",
        );
      }

      const { deployAndSaveEnclaveToken } = await import(
        "../scripts/deployAndSave/enclaveToken"
      );
      const { enclaveToken } = await deployAndSaveEnclaveToken({ hre });

      const { deployAndSaveMockStableToken } = await import(
        "../scripts/deployAndSave/mockStableToken"
      );
      const { mockStableToken } = await deployAndSaveMockStableToken({
        hre,
      });

      const [signer] = await ethers.getSigners();
      const enclaveTokenContract = enclaveToken.connect(signer);
      const mockUSDCContract = mockStableToken.connect(signer);

      try {
        console.log(`Minting tokens for: ${ciphernodeAddress}`);

        console.log(`Minting ${enclAmount} ENCL...`);
        const enclTx = await enclaveTokenContract.mintAllocation(
          ciphernodeAddress,
          ethers.parseEther(enclAmount),
          "Ciphernode allocation",
        );
        await enclTx.wait();
        console.log(`${enclAmount} ENCL minted`);

        console.log(`Minting ${usdcAmount} USDC...`);
        const usdcTx = await mockUSDCContract.mint(
          ciphernodeAddress,
          ethers.parseUnits(usdcAmount, 6),
        );
        await usdcTx.wait();
        console.log(`${usdcAmount} USDC minted`);

        const enclBalance =
          await enclaveTokenContract.balanceOf(ciphernodeAddress);
        const usdcBalance = await mockUSDCContract.balanceOf(ciphernodeAddress);

        console.log("\n=== Token Balances ===");
        console.log(`ENCL: ${ethers.formatEther(enclBalance)}`);
        console.log(`USDC: ${ethers.formatUnits(usdcBalance, 6)}`);

        const transfersRestricted =
          await enclaveTokenContract.transfersRestricted();
        if (transfersRestricted) {
          console.log("Allowing EnclaveToken to be transferrable...");
          const transferEnabledTx =
            await enclaveTokenContract.setTransferRestriction(false);
          await transferEnabledTx.wait();
          console.log("EnclaveToken transfers are now enabled");
        }
      } catch (error) {
        console.error("Token minting failed:", error);
        throw error;
      }
    },
  }))
  .build();

export const ciphernodeAdminAdd = task(
  "ciphernode:admin-add",
  "Register a ciphernode using admin privileges (for testing/setup)",
)
  .addOption({
    name: "ciphernodeAddress",
    description: "address of ciphernode to register",
    defaultValue: ZeroAddress,
  })
  .addOption({
    name: "adminPrivateKey",
    description:
      "private key of admin wallet (optional, uses anvil first key if not provided)",
    defaultValue: "",
  })
  .addOption({
    name: "licenseBondAmount",
    description:
      "amount of ENCL to bond (in ether units, e.g., 1000 for 1000 ENCL)",
    defaultValue: "1000",
  })
  .addOption({
    name: "ticketAmount",
    description:
      "amount of USDC for tickets (in USDC units, e.g., 1000 for 1000 USDC)",
    defaultValue: "1000",
  })
  .setAction(async () => ({
    default: async (
      { ciphernodeAddress, adminPrivateKey, licenseBondAmount, ticketAmount },
      hre,
    ) => {
      const connection = await hre.network.connect();
      const { ethers } = connection;

      if (ciphernodeAddress === ZeroAddress) {
        throw new Error(
          "Ciphernode address is required. Use --ciphernode-address option.",
        );
      }

      let adminWallet;
      if (adminPrivateKey) {
        adminWallet = new ethers.Wallet(adminPrivateKey, ethers.provider);
      } else {
        const anvilFirstKey =
          "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        adminWallet = new ethers.Wallet(anvilFirstKey, ethers.provider);
      }

      console.log(`Admin wallet: ${adminWallet.address}`);
      console.log(`Registering ciphernode: ${ciphernodeAddress}`);

      const { deployAndSaveBondingRegistry } = await import(
        "../scripts/deployAndSave/bondingRegistry"
      );
      const { bondingRegistry } = await deployAndSaveBondingRegistry({ hre });

      const { deployAndSaveEnclaveToken } = await import(
        "../scripts/deployAndSave/enclaveToken"
      );
      const { enclaveToken } = await deployAndSaveEnclaveToken({ hre });

      const { deployAndSaveMockStableToken } = await import(
        "../scripts/deployAndSave/mockStableToken"
      );
      const { mockStableToken: mockUSDC } = await deployAndSaveMockStableToken({
        hre,
      });

      const enclaveTokenConnected = enclaveToken.connect(adminWallet);
      const mockUSDCConnected = mockUSDC.connect(adminWallet);

      const ticketTokenAddress = await bondingRegistry.ticketToken();

      try {
        const licenseBondWei = ethers.parseEther(licenseBondAmount);
        const ticketAmountWei = ethers.parseUnits(ticketAmount, 6);

        console.log("Step 1: Minting and transferring ENCL to ciphernode...");

        const enclTx = await enclaveTokenConnected.mintAllocation(
          adminWallet.address,
          licenseBondWei,
          "Admin allocation for ciphernode registration",
        );
        await enclTx.wait();

        const transferTx = await enclaveTokenConnected.transfer(
          ciphernodeAddress,
          licenseBondWei,
        );
        await transferTx.wait();
        console.log(`${licenseBondAmount} ENCL transferred to ciphernode`);

        console.log("Step 2: Minting USDC to admin...");
        const usdcTx = await mockUSDCConnected.mint(
          adminWallet.address,
          ticketAmountWei,
        );
        await usdcTx.wait();
        console.log(`${ticketAmount} USDC minted to admin`);

        console.log(
          "Step 3: Impersonating ciphernode for license operations...",
        );
        await connection.provider.request({
          method: "hardhat_impersonateAccount",
          params: [ciphernodeAddress],
        });

        await connection.provider.request({
          method: "hardhat_setBalance",
          params: [ciphernodeAddress, "0x1000000000000000000000"],
        });

        const ciphernodeSigner = await ethers.getSigner(ciphernodeAddress);
        const enclaveTokenAsCiphernode = enclaveToken.connect(ciphernodeSigner);
        const bondingRegistryAsCiphernode =
          bondingRegistry.connect(ciphernodeSigner);

        const approveTx = await enclaveTokenAsCiphernode.approve(
          await bondingRegistry.getAddress(),
          licenseBondWei,
        );
        await approveTx.wait();

        const bondTx =
          await bondingRegistryAsCiphernode.bondLicense(licenseBondWei);
        await bondTx.wait();
        console.log(`License bonded: ${licenseBondAmount} ENCL`);

        const registerTx = await bondingRegistryAsCiphernode.registerOperator();
        await registerTx.wait();
        console.log(
          "Operator registered (automatically added to CiphernodeRegistry)",
        );

        await connection.provider.request({
          method: "hardhat_stopImpersonatingAccount",
          params: [ciphernodeAddress],
        });

        console.log("Step 4: Adding ticket balance via admin...");

        const approveUsdcTx = await mockUSDCConnected.approve(
          ticketTokenAddress,
          ticketAmountWei,
        );
        await approveUsdcTx.wait();

        await connection.provider.request({
          method: "hardhat_impersonateAccount",
          params: [ciphernodeAddress],
        });

        await connection.provider.request({
          method: "hardhat_setBalance",
          params: [ciphernodeAddress, "0x1000000000000000000000"],
        });

        const ciphernodeSigner2 = await ethers.getSigner(ciphernodeAddress);
        const bondingRegistryAsCiphernode2 =
          bondingRegistry.connect(ciphernodeSigner2);

        const usdcTransferTx = await mockUSDCConnected.transfer(
          ciphernodeAddress,
          ticketAmountWei,
        );
        await usdcTransferTx.wait();

        const mockUSDCAsCiphernode = mockUSDC.connect(ciphernodeSigner2);
        const approveUsdcAsCiphernodeTx = await mockUSDCAsCiphernode.approve(
          ticketTokenAddress,
          ticketAmountWei,
        );
        await approveUsdcAsCiphernodeTx.wait();

        const addTicketTx =
          await bondingRegistryAsCiphernode2.addTicketBalance(ticketAmountWei);
        await addTicketTx.wait();
        console.log(`Ticket balance added: ${ticketAmount} USDC worth`);

        await connection.provider.request({
          method: "hardhat_stopImpersonatingAccount",
          params: [ciphernodeAddress],
        });

        const isRegistered =
          await bondingRegistry.isRegistered(ciphernodeAddress);
        const isActive = await bondingRegistry.isActive(ciphernodeAddress);
        const licenseBond =
          await bondingRegistry.getLicenseBond(ciphernodeAddress);
        const ticketBalance =
          await bondingRegistry.getTicketBalance(ciphernodeAddress);

        console.log("\n=== Registration Complete ===");
        console.log(`Ciphernode: ${ciphernodeAddress}`);
        console.log(`Registered: ${isRegistered}`);
        console.log(`Active: ${isActive}`);
        console.log(`License Bond: ${ethers.formatEther(licenseBond)} ENCL`);
        console.log(
          `Ticket Balance: ${ethers.formatUnits(ticketBalance, 6)} USDC worth`,
        );
      } catch (error) {
        console.error("Admin registration failed:", error);
        throw error;
      }
    },
  }))
  .build();

export const ciphernodeSiblings = task(
  "ciphernode:siblings",
  "Get the sibling of a ciphernode in the registry",
)
  .addOption({
    name: "ciphernodeAddress",
    description: "address of ciphernode to get siblings for",
    defaultValue: ZeroAddress,
  })
  .addOption({
    name: "ciphernodeAddresses",
    description:
      "comma separated addresses of ciphernodes in the order they were added to the registry",
    defaultValue: ZeroAddress,
  })
  .setAction(async () => ({
    default: async ({ ciphernodeAddress, ciphernodeAddresses }, _) => {
      const hash = (a: bigint, b: bigint) => poseidon2([a, b]);
      const tree = new LeanIMT(hash);

      const addresses = ciphernodeAddresses.split(",");

      for (const address of addresses) {
        tree.insert(BigInt(address));
      }

      const index = tree.indexOf(BigInt(ciphernodeAddress));
      const { siblings } = tree.generateProof(index);

      console.log(`Siblings for ${ciphernodeAddress}: ${siblings}`);
    },
  }))
  .build();
