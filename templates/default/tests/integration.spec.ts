import {
  calculateStartWindow,
  DEFAULT_COMPUTE_PROVIDER_PARAMS,
  DEFAULT_E3_CONFIG,
  EnclaveEventType,
  EnclaveSDK,
  encodeBfvParams,
  encodeComputeProviderParams,
  RegistryEventType,
  AllEventTypes,
  EnclaveEvent,
  encryptNumber,
} from "@gnosis-guild/enclave/sdk";
import { hexToBytes } from "viem";
import { E3 } from "@gnosis-guild/enclave/sdk/types.js";
import assert from "assert";

export function getContractAddresses() {
  return {
    enclave: process.env.ENCLAVE_ADDRESS as `0x${string}`,
    ciphernodeRegistry: process.env.REGISTRY_ADDRESS as `0x${string}`,
    filterRegistry: process.env.FILTER_REGISTRY_ADDRESS as `0x${string}`,
    e3Program: process.env.E3_PROGRAM_ADDRESS as `0x${string}`,
  };
}

type E3Shared = {
  e3Id: bigint;
  e3Program: string;
  e3: E3;
  filter: string;
};

type E3StateRequested = E3Shared & {
  type: "requested";
};

type E3StatePublished = E3Shared & {
  type: "committee_published";
  publicKey: `0x${string}`;
};

type E3StateActivated = E3Shared & {
  type: "activated";
  publicKey: `0x${string}`;
  expiration: bigint;
};

type E3StateOutputPublished = E3Shared & {
  type: "output_published";
  plaintextOutput: string;
};

type E3State =
  | E3StateRequested
  | E3StatePublished
  | E3StateActivated
  | E3StateOutputPublished;

async function setupEventListeners(
  sdk: EnclaveSDK,
  store: Map<bigint, E3State>,
) {
  async function waitForEvent<T extends AllEventTypes>(
    type: T,
    trigger?: () => Promise<void>,
  ): Promise<EnclaveEvent<T>> {
    return new Promise((resolve) => {
      sdk.once(type, resolve);
      trigger && trigger();
    });
  }

  sdk.onEnclaveEvent(EnclaveEventType.E3_REQUESTED, (event) => {
    const id = event.data.e3Id;

    if (store.has(id)) {
      throw new Error("E3 has already been requested ");
    }

    store.set(event.data.e3Id, {
      type: "requested",
      ...event.data,
    });
  });

  sdk.onEnclaveEvent(RegistryEventType.COMMITTEE_PUBLISHED, (event) => {
    const id = event.data.e3Id;

    const state = store.get(id);

    if (!state) {
      throw new Error(`State for ID '${id}'not found.`);
    }

    if (state.type !== "requested") {
      throw new Error(`State must be in the ${state.type} state`);
    }

    store.set(id, {
      publicKey: event.data.publicKey as `0x${string}`,
      ...state,
      type: "committee_published",
    });
  });

  sdk.onEnclaveEvent(EnclaveEventType.E3_ACTIVATED, (event) => {
    const id = event.data.e3Id;
    const state = store.get(id);

    if (!state) {
      throw new Error(`State for ID '${id}' not found.`);
    }

    if (state.type !== "committee_published") {
      throw new Error(`State must be in the ${state.type} state`);
    }

    store.set(id, {
      ...state,
      expiration: event.data.expiration,
      publicKey: event.data.committeePublicKey as `0x${string}`,
      type: "activated",
    });
  });

  sdk.onEnclaveEvent(EnclaveEventType.PLAINTEXT_OUTPUT_PUBLISHED, (event) => {
    const id = event.data.e3Id;
    const state = store.get(id);

    if (!state) {
      throw new Error(`State for ID '${id}' not found.`);
    }

    if (state.type !== "committee_published") {
      throw new Error(`State must be in the ${state.type} state`);
    }

    store.set(id, {
      ...state,
      plaintextOutput: event.data.plaintextOutput,
      type: "output_published",
    });
  });

  return { waitForEvent };
}

async function main() {
  console.log("Testing...");

  const contracts = getContractAddresses();

  const store = new Map<bigint, E3State>();
  const sdk = EnclaveSDK.create({
    chainId: 31337,
    contracts: {
      enclave: contracts.enclave,
      ciphernodeRegistry: contracts.ciphernodeRegistry,
    },
    rpcUrl: "ws://localhost:8545",
    privateKey:
      "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
  });

  const { waitForEvent } = await setupEventListeners(sdk, store);

  const threshold: [number, number] = [
    DEFAULT_E3_CONFIG.threshold_min,
    DEFAULT_E3_CONFIG.threshold_max,
  ];
  const startWindow = calculateStartWindow(60); // 1 minute
  const duration = BigInt(60); // 1 minute
  const e3ProgramParams = encodeBfvParams();
  const computeProviderParams = encodeComputeProviderParams(
    DEFAULT_COMPUTE_PROVIDER_PARAMS,
  );

  let state;
  let event;

  // REQUEST phase
  await waitForEvent(EnclaveEventType.E3_REQUESTED, async () => {
    await sdk.requestE3({
      filter: contracts.filterRegistry,
      threshold,
      startWindow,
      duration,
      e3Program: contracts.e3Program,
      e3ProgramParams,
      computeProviderParams,
      value: BigInt("1000000000000000"), // 0.001 ETH
    });
  });

  state = store.get(0n);
  assert(state);
  assert.strictEqual(state.e3Id, 0n);
  assert.strictEqual(state.filter, contracts.filterRegistry);
  assert.strictEqual(state.type, "requested");

  // Ciphernodes will publish a public key within the COMMITTEE_PUBLISHED event
  event = await waitForEvent(RegistryEventType.COMMITTEE_PUBLISHED);

  state = store.get(0n);
  assert(state);
  assert.strictEqual(state.type, "committee_published");
  assert.strictEqual(state.publicKey, event.data.publicKey);

  let { e3Id, publicKey } = state;

  // ACTIVATION phase
  event = await waitForEvent(EnclaveEventType.E3_ACTIVATED, async () => {
    await sdk.activateE3(e3Id, publicKey);
  });

  state = store.get(0n);
  assert(state);
  assert.strictEqual(state.type, "activated");

  // INPUT PUBLISHING phase
  const num1 = 123n;
  const num2 = 210n;
  const publicKeyBytes = hexToBytes(state.publicKey);
  const enc1 = encryptNumber(num1, publicKeyBytes);
  const enc2 = encryptNumber(num2, publicKeyBytes);

  await waitForEvent(EnclaveEventType.INPUT_PUBLISHED, async () => {
    await sdk.publishInput(
      e3Id,
      `0x${Array.from(enc1, (b) => b.toString(16).padStart(2, "0")).join("")}` as `0x${string}`,
    );
  });
  await waitForEvent(EnclaveEventType.INPUT_PUBLISHED, async () => {
    const hash2 = await sdk.publishInput(
      e3Id,
      `0x${Array.from(enc2, (b) => b.toString(16).padStart(2, "0")).join("")}` as `0x${string}`,
    );
  });

  const plaintextEvent = await waitForEvent(
    EnclaveEventType.PLAINTEXT_OUTPUT_PUBLISHED,
  );

  assert.strictEqual(plaintextEvent.data.plaintextOutput, "0x12345678");

  console.log("It worked");

  process.exit(0);
}

main()
  .then(() => console.log("Test successful"))
  .catch((err) => {
    console.log("");
    console.log("  ❌ Test failed ");
    console.log("");
    console.log(err);
    process.exit(1);
  });
