import express, { Request, Response } from "express";
import { EnclaveSDK } from "@gnosis-guild/enclave/sdk";
import { handleRpc } from "typed-rpc/server";

function validateHex(value: string, length?: number, name = "value"): boolean {
  if (!value?.startsWith("0x") || !/^[a-fA-F0-9]*$/.test(value.slice(2))) {
    throw new Error(`${name} must be valid hex`);
  }
  if (length && value.slice(2).length !== length) {
    throw new Error(`${name} must be ${length} hex chars`);
  }
  return true;
}

function ensureEnv(key: string): string {
  const value = process.env[key];
  if (!value) {
    throw new Error(`Missing required env var: ${key}`);
  }
  return value;
}

function getCheckedEnvVars() {
  return {
    RPC_URL: ensureEnv("RPC_URL"),
    ENCLAVE_CONTRACT: ensureEnv("ENCLAVE_ADDRESS"),
    CIPHERNODE_REGISTRY_CONTRACT: ensureEnv("REGISTRY_ADDRESS"),
    PRIVATE_KEY: ensureEnv("PRIVATE_KEY"),
    CHAIN_ID: parseInt(ensureEnv("CHAIN_ID")),
  };
}

async function createPrivateSDK() {
  const {
    CHAIN_ID,
    PRIVATE_KEY,
    CIPHERNODE_REGISTRY_CONTRACT,
    ENCLAVE_CONTRACT,
    RPC_URL,
  } = getCheckedEnvVars();

  if (!isSupportedChain(CHAIN_ID)) {
    throw new Error(`Unsupported CHAIN_ID: ${CHAIN_ID}`);
  }

  const sdk = EnclaveSDK.create({
    rpcUrl: RPC_URL,
    privateKey: PRIVATE_KEY as `0x${string}`,
    contracts: {
      enclave: ENCLAVE_CONTRACT as `0x${string}`,
      ciphernodeRegistry: CIPHERNODE_REGISTRY_CONTRACT as `0x${string}`,
    },
    chainId: CHAIN_ID,
  });

  await sdk.initialize();
  return sdk;
}

function isValidHexString(value: string): value is `0x${string}` {
  return value.startsWith("0x") && /^0x[a-fA-F0-9]*$/.test(value);
}

function isSupportedChain(value: any): value is keyof typeof EnclaveSDK.chains {
  return value in EnclaveSDK.chains;
}

// This should check if we are missing env vars and throw if any are missing.
getCheckedEnvVars();

const app = express();

app.use(express.json());

app.post("/", (req: Request, res: Response) => {
  handleRpc(req.body, {
    // This is called before a computation is attempted. You can use it to prevent unecessary computation.
    shouldCompute(e3Params: string, ciphertextInputs: Array<[string, number]>) {
      console.log(
        `shouldCompute(e3Id:${e3Params},ciphertextInputs.length:${ciphertextInputs.length})`,
      );
      return ciphertextInputs.length > 1;
    },

    // This is called after computation has occurred
    async processOutput(e3Id: number, proof: string, ciphertext: string) {
      console.log(
        `processOutput(e3Id:${e3Id},proof:${proof},ciphertext:${ciphertext})`,
      );
      if (!isValidHexString(ciphertext) || !isValidHexString(proof)) {
        throw new Error("Input is not valid");
      }
      const sdk = await createPrivateSDK();

      await sdk.publishCiphertextOutput(BigInt(e3Id), ciphertext, proof);

      return 0;
    },

    // This informs the caller of what methods are available on this server
    capabilities() {
      return [
        "shouldCompute", // optional
        "processOutput", // mandatory
      ];
    },
  }).then((result) => res.json(result));
});
const PORT = 8080;
app.listen(PORT, () => {
  console.log(`Server is listening on ${8080}`);
});
