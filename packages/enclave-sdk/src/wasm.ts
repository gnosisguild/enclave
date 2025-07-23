// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { encrypt_number } from "@gnosis-guild/e3-wasm";
import init from "@gnosis-guild/e3-wasm/init";

export async function encryptNumber(
  data: bigint,
  public_key: Uint8Array,
): Promise<Uint8Array> {
  await init();
  return encrypt_number(data, public_key);
}
