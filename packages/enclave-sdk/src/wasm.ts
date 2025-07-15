import init, { encrypt_number } from "@gnosis-guild/e3-wasm";

export async function encryptNumber(
  data: bigint,
  public_key: Uint8Array,
): Promise<Uint8Array> {
  await init();
  return encrypt_number(data, public_key);
}
