import { encrypt_number } from "@gnosis-guild/e3-wasm";

export function encryptNumber(
  data: bigint,
  public_key: Uint8Array,
): Uint8Array {
  return encrypt_number(data, public_key);
}
