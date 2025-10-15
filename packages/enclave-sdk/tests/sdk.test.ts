// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { describe, expect, it } from "vitest";
import fs from "fs/promises";
import path from "path";
import { CompiledCircuit } from "@noir-lang/noir_js";

import { EnclaveSDK } from "../src/enclave-sdk";
import { zeroAddress } from "viem";
import { FheProtocol } from "../src/types";
import demoCircuit from "./fixtures/demo_circuit.json";

describe("encryptNumber", () => {
  describe("bfv", () => {
    // create SDK with default config
    const sdk = EnclaveSDK.create({
      chainId: 31337,
      contracts: {
        enclave: zeroAddress,
        ciphernodeRegistry: zeroAddress,
      },
      rpcUrl: "",
      privateKey:
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
      protocol: FheProtocol.BFV,
    });

    it("should encrypt a number without crashing in a node environent", async () => {
      const buffer = await fs.readFile(
        path.resolve(__dirname, "./fixtures/pubkey.bin"),
      );
      const value = await sdk.encryptNumber(10n, Uint8Array.from(buffer));
      expect(value).to.be.an.instanceof(Uint8Array);
      expect(value.length).to.equal(27_674);
      // TODO: test the encryption is correct
    });
    it("should encrypt a number and generate a proof without crashing in a node environent", async () => {
      const buffer = await fs.readFile(
        path.resolve(__dirname, "./fixtures/pubkey.bin"),
      );

      const value = await sdk.encryptNumberAndGenProof(1n, Uint8Array.from(buffer), demoCircuit as unknown as CompiledCircuit);
      
      expect(value).to.be.an.instanceof(Object);
      expect(value.encryptedVote).to.be.an.instanceof(Uint8Array);
      expect(value.proof).to.be.an.instanceOf(Object)
    }, 9999999);

    it("should encrypt a vecor of numbers without crashing in a node environent", async () => {
      const buffer = await fs.readFile(
        path.resolve(__dirname, "./fixtures/pubkey.bin"),
      );
      const value = await sdk.encryptVector(new BigUint64Array([1n, 2n]), Uint8Array.from(buffer));
      expect(value).to.be.an.instanceof(Uint8Array);
      expect(value.length).to.equal(27_674);
    });

    it("should encrypt a vector and generate a proof without crashing in a node environent", async () => {
      const buffer = await fs.readFile(
        path.resolve(__dirname, "./fixtures/pubkey.bin"),
      );

      const value = await sdk.encryptVectorAndGenProof(new BigUint64Array([1n, 2n]), Uint8Array.from(buffer), demoCircuit as unknown as CompiledCircuit);
      
      expect(value).to.be.an.instanceof(Object);
      expect(value.encryptedVote).to.be.an.instanceof(Uint8Array);
      expect(value.proof).to.be.an.instanceOf(Object)
    }, 9999999);
  });
});
