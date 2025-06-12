import { defineWalletSetup } from "@synthetixio/synpress";
import { MetaMask } from "@synthetixio/synpress/playwright";

const SEED_PHRASE =
  "test test test test test test test test test test test junk";
const PASSWORD = "Tester@1234";

console.log("🔐 [Wallet Setup] Starting MetaMask wallet setup");

export default defineWalletSetup(PASSWORD, async (context, walletPage) => {
  console.log("🔐 [Wallet Setup] Importing wallet...");
  const metamask = new MetaMask(context, walletPage, PASSWORD);
  await metamask.importWallet(SEED_PHRASE);

  console.log("🌐 [Wallet Setup] Adding custom network...");
  const customNetwork = {
    name: "localwallet",
    rpcUrl: "http://localhost:8545",
    chainId: 31337,
    symbol: "ETH",
  };
  await metamask.addNetwork(customNetwork);

  console.log("✅ [Wallet Setup] Wallet setup complete");
});
