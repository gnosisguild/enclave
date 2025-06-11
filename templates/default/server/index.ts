import express, { Request, Response } from "express";
import {
  EnclaveSDK,
  EnclaveEventType,
  type E3ActivatedData,
  type InputPublishedData,
  type E3RequestedData,
} from "@gnosis-guild/enclave/sdk";

interface E3Session {
  e3Id: bigint;
  expiration: bigint;
  e3ProgramParams?: string;
  inputs: Array<{ data: string; index: bigint }>;
  isProcessing: boolean;
  isCompleted: boolean;
}

const e3Sessions = new Map<string, E3Session>();

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
    PROGRAM_RUNNER_URL:
      process.env.PROGRAM_RUNNER_URL || "http://127.0.0.1:13151",
    CALLBACK_URL: process.env.CALLBACK_URL || "http://127.0.0.1:8080",
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

async function callFheRunner(
  e3Id: bigint,
  params: string,
  ciphertextInputs: Array<[string, number]>,
): Promise<void> {
  const { PROGRAM_RUNNER_URL, CALLBACK_URL } = getCheckedEnvVars();

  const payload = {
    e3_id: Number(e3Id),
    params,
    ciphertext_inputs: ciphertextInputs,
    callback_url: CALLBACK_URL,
  };
  console.log("payload:");
  console.log(JSON.stringify(payload));

  const response = await fetch(`${PROGRAM_RUNNER_URL}/run_compute`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    throw new Error(
      `FHE Runner failed: ${response.status} ${response.statusText}`,
    );
  }

  const result = await response.json();
  console.log(`✓ FHE Runner accepted E3 ${e3Id}:`, result);
}

async function processE3Session(e3Id: bigint): Promise<void> {
  const sessionKey = e3Id.toString();
  const session = e3Sessions.get(sessionKey);

  if (!session || session.isProcessing || session.isCompleted) {
    return;
  }

  console.log(
    `📊 Processing E3 session ${e3Id} with ${session.inputs.length} inputs`,
  );

  try {
    session.isProcessing = true;

    if (session.inputs.length <= 1) {
      console.log(
        `⏭️  Skipping E3 ${e3Id}: not enough inputs (${session.inputs.length})`,
      );
      session.isCompleted = true;
      return;
    }

    let e3ProgramParams = session.e3ProgramParams;
    if (!e3ProgramParams) {
      const sdk = await createPrivateSDK();
      const e3Details = (await sdk.getE3(e3Id)) as any;
      e3ProgramParams = e3Details.e3ProgramParams;
      session.e3ProgramParams = e3ProgramParams;
    }

    const ciphertextInputs: Array<[string, number]> = session.inputs.map(
      (input) => [input.data, Number(input.index)],
    );

    console.log(`🔄 Calling FHE runner for E3 ${e3Id}...`);
    await callFheRunner(e3Id, e3ProgramParams!, ciphertextInputs);

    console.log(`✅ E3 ${e3Id} sent to FHE runner - awaiting callback`);
  } catch (error) {
    console.error(`❌ Error processing E3 ${e3Id}:`, error);
    session.isProcessing = false;
  }
}

async function setupEventListeners() {
  const sdk = await createPrivateSDK();

  console.log("📡 Setting up event listeners...");

  sdk.onEnclaveEvent(EnclaveEventType.E3_ACTIVATED, async (event) => {
    const data = event.data as E3ActivatedData;
    const e3Id = data.e3Id;
    const expiration = data.expiration;

    console.log(`🎯 E3 Activated: ${e3Id}, expiration: ${expiration}`);

    const sessionKey = e3Id.toString();
    if (!e3Sessions.has(sessionKey)) {
      const e3 = await sdk.getE3(e3Id);
      e3Sessions.set(sessionKey, {
        e3Id,
        e3ProgramParams: e3.e3ProgramParams,
        expiration,
        inputs: [],
        isProcessing: false,
        isCompleted: false,
      });
    }

    const currentTime = BigInt(Math.floor(Date.now() / 1000));
    const sleepSeconds =
      expiration > currentTime ? Number(expiration - currentTime) : 0;

    if (sleepSeconds > 0) {
      console.log(
        `⏰ Scheduling E3 ${e3Id} processing in ${sleepSeconds} seconds...`,
      );
      setTimeout(async () => {
        await processE3Session(e3Id);
      }, sleepSeconds * 1000);
    } else {
      console.log(`⚡ E3 ${e3Id} already expired, processing immediately...`);
      await processE3Session(e3Id);
    }
  });

  sdk.onEnclaveEvent(EnclaveEventType.INPUT_PUBLISHED, async (event) => {
    const data = event.data as InputPublishedData;
    const e3Id = data.e3Id;

    console.log(`📝 Input Published for E3 ${e3Id}: index ${data.index}`);

    const sessionKey = e3Id.toString();
    const session = e3Sessions.get(sessionKey);

    if (session) {
      session.inputs.push({
        data: data.data,
        index: data.index,
      });
      console.log(`📊 E3 ${e3Id} now has ${session.inputs.length} inputs`);
    } else {
      console.warn(`⚠️  Received input for unknown E3 session: ${e3Id}`);
    }
  });

  console.log("✅ Event listeners set up successfully");
}

function isValidHexString(value: string): value is `0x${string}` {
  return value.startsWith("0x") && /^0x[a-fA-F0-9]*$/.test(value);
}

function isSupportedChain(value: any): value is keyof typeof EnclaveSDK.chains {
  return value in EnclaveSDK.chains;
}

const app = express();
app.use(express.json());

app.post("/", async (req: Request, res: Response) => {
  try {
    console.log("📨 Webhook received:", req.body);

    const { e3_id, ciphertext, proof } = req.body;

    if (!e3_id || !ciphertext || !proof) {
      res
        .status(400)
        .json({ error: "Missing required fields: e3_id, ciphertext, proof" });
      return;
    }

    if (!isValidHexString(ciphertext) || !isValidHexString(proof)) {
      res
        .status(400)
        .json({ error: "ciphertext and proof must be valid hex strings" });
      return;
    }

    console.log(`🔄 Publishing output for E3 ${e3_id}...`);

    const sdk = await createPrivateSDK();
    await sdk.publishCiphertextOutput(BigInt(e3_id), ciphertext, proof);

    // Mark session as completed
    const sessionKey = e3_id.toString();
    const session = e3Sessions.get(sessionKey);
    if (session) {
      session.isCompleted = true;
      session.isProcessing = false;
      console.log(`✅ Successfully completed E3 ${e3_id}`);
    }

    res.json({ status: "success", e3_id });
  } catch (error) {
    console.error("❌ Webhook processing failed:", error);
    res.status(500).json({ error: "Internal server error" });
  }
});

app.get("/sessions", (req: Request, res: Response) => {
  const sessions = Array.from(e3Sessions.entries()).map(([key, session]) => ({
    e3Id: key,
    expiration: session.expiration.toString(),
    inputCount: session.inputs.length,
    isProcessing: session.isProcessing,
    isCompleted: session.isCompleted,
  }));
  res.json(sessions);
});

async function startServer() {
  try {
    await setupEventListeners();

    const PORT = process.env.PORT ? parseInt(process.env.PORT) : 8080;
    app.listen(PORT, () => {
      console.log(`🚀 Enclave Server listening on port ${PORT}`);
      console.log(`📡 Event listeners active`);
      console.log(`📊 Sessions: http://localhost:${PORT}/sessions`);
    });
  } catch (error) {
    console.error("❌ Failed to start server:", error);
    process.exit(1);
  }
}

startServer().catch(console.error);
