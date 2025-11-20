// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import payload from './payload.json'
import { callFheRunner } from './runner'

export async function handleTestInteraction() {
  let e3Id = BigInt(payload.e3_id)
  let params = payload.params
  let ciphertextInputs = payload.ciphertext_inputs as Array<[string, number]>
  await callFheRunner(e3Id, params, ciphertextInputs)
}
