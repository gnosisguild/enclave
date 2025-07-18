import { encrypt_number } from "@gnosis-guild/e3-wasm";
import init from "@gnosis-guild/e3-wasm/init";

export async function encryptNumber(
  data: bigint,
  public_key: Uint8Array,
): Promise<Uint8Array> {
  await init();
  return encrypt_number(data, public_key);
}
