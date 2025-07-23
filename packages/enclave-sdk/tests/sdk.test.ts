// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { describe, expect, it } from "vitest";
import fs from "fs/promises";
import path from "path";

import { encryptNumber } from "../src";

describe("encryptNumber", () => {
  it("should encrypt a number without crashing in a node environent", async () => {
    const buffer = await fs.readFile(
      path.resolve(__dirname, "./fixtures/pubkey.bin"),
    );
    const value = await encryptNumber(10n, Uint8Array.from(buffer));
    expect(value).to.be.an.instanceof(Uint8Array);
    expect(value.length).to.equal(27_674);
    // TODO: test the encryption is correct
  });
});
