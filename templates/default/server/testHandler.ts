import payload from "./payload.json";
import { callFheRunner } from "./runner";

export async function handleTestInteraction() {
  let e3Id = BigInt(payload.e3_id);
  let params = payload.params;
  let ciphertextInputs = payload.ciphertext_inputs as Array<[string, number]>;
  await callFheRunner(e3Id, params, ciphertextInputs);
}
