// Import necessary Synpress modules
import { defineWalletSetup } from "@synthetixio/synpress";
import { MetaMask } from "@synthetixio/synpress/playwright";

// Define a test seed phrase and password
const SEED_PHRASE =
  "test test test test test test test test test test test junk";
const PASSWORD = "Tester@1234";

export default defineWalletSetup(PASSWORD, async (context, walletPage) => {
  const metamask = new MetaMask(context, walletPage, PASSWORD);
  await metamask.importWallet(SEED_PHRASE);
  const customNetwork = {
    name: "localwallet",
    rpcUrl: "http://localhost:8545",
    chainId: 31337,
    symbol: "ETH",
  };
  await metamask.addNetwork(customNetwork);
});
