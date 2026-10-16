// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { Barretenberg, UltraHonkBackend, type ProofData } from '@aztec/bb.js'
import userDataEncryptionCt0Circuit from '../../../../circuits/bin/threshold/target/user_data_encryption_ct0.json'
import userDataEncryptionCt1Circuit from '../../../../circuits/bin/threshold/target/user_data_encryption_ct1.json'
import userDataEncryptionCircuit from '../../../../circuits/bin/threshold/target/user_data_encryption.json'
// The `noir-recursive-no-zk` keys the circuit build writes next to the bytecode. bb.js's
// `generateRecursiveProofArtifacts` would rebuild each circuit to recompute them on every encryption.
import userDataEncryptionCt0Vk from '../../../../circuits/bin/threshold/target/user_data_encryption_ct0.vk_recursive.json'
import userDataEncryptionCt1Vk from '../../../../circuits/bin/threshold/target/user_data_encryption_ct1.vk_recursive.json'
import { CompiledCircuit, Noir } from '@noir-lang/noir_js'
import { proofToFields } from '../utils'

import type { CircuitInputs } from './user-data-encryption'

export const proveUserDataEncryption = async (circuitInputs: CircuitInputs): Promise<ProofData> => {
  // The outer circuit needs 2^21 SRS points. Pass the size to `new`: on the WASM backend that
  // browsers use, a later `initSRSChonk` call does not enlarge the SRS that `new` loads.
  const api = await Barretenberg.new({ srsSize: 2 ** 21 })

  try {
    const { witness: userDataEncryptionCt0Witness } = await executeCircuit(userDataEncryptionCt0Circuit as CompiledCircuit, {
      pk0is: circuitInputs.pk0is,
      ct0is: circuitInputs.ct0is,
      u: circuitInputs.u,
      e0: circuitInputs.e0,
      k1: circuitInputs.k1,
      r: circuitInputs.r,
    })
    // ct1 hard-codes the CRS as `pk1`, so the key's second component is not an input.
    const { witness: userDataEncryptionCt1Witness } = await executeCircuit(userDataEncryptionCt1Circuit as CompiledCircuit, {
      ct1is: circuitInputs.ct1is,
      u: circuitInputs.u,
      e1: circuitInputs.e1,
      r_ct1: circuitInputs.r_ct1,
    })

    const userDataEncryptionCt0Backend = new UltraHonkBackend((userDataEncryptionCt0Circuit as CompiledCircuit).bytecode, api)
    const userDataEncryptionCt1Backend = new UltraHonkBackend((userDataEncryptionCt1Circuit as CompiledCircuit).bytecode, api)

    const { proof: userDataEncryptionCt0Proof, publicInputs: userDataEncryptionCt0PublicInputs } =
      await userDataEncryptionCt0Backend.generateProof(userDataEncryptionCt0Witness, {
        verifierTarget: 'noir-recursive-no-zk',
      })
    const { proof: userDataEncryptionCt1Proof, publicInputs: userDataEncryptionCt1PublicInputs } =
      await userDataEncryptionCt1Backend.generateProof(userDataEncryptionCt1Witness, {
        verifierTarget: 'noir-recursive-no-zk',
      })

    const userDataEncryptionCt0Artifacts: RecursiveVk = userDataEncryptionCt0Vk
    const userDataEncryptionCt1Artifacts: RecursiveVk = userDataEncryptionCt1Vk

    const { witness: userDataEncryptionWitness } = await executeCircuit(userDataEncryptionCircuit as CompiledCircuit, {
      ct0_verification_key: userDataEncryptionCt0Artifacts.vkAsFields,
      ct0_proof: proofToFields(userDataEncryptionCt0Proof),
      ct0_public_inputs: userDataEncryptionCt0PublicInputs,
      ct0_key_hash: userDataEncryptionCt0Artifacts.vkHash,
      ct1_verification_key: userDataEncryptionCt1Artifacts.vkAsFields,
      ct1_proof: proofToFields(userDataEncryptionCt1Proof),
      ct1_public_inputs: userDataEncryptionCt1PublicInputs,
      ct1_key_hash: userDataEncryptionCt1Artifacts.vkHash,
    })

    const userDataEncryptionBackend = new UltraHonkBackend((userDataEncryptionCircuit as CompiledCircuit).bytecode, api)

    return await userDataEncryptionBackend.generateProof(userDataEncryptionWitness, {
      verifierTarget: 'noir-recursive-no-zk',
    })
  } finally {
    // `destroy` returns a promise; await it so the proof settles after cleanup completes.
    await api.destroy()
  }
}

/** A verification key in the shape bb.js's `generateRecursiveProofArtifacts` returns it. */
type RecursiveVk = { vkAsFields: string[]; vkHash: string }

const executeCircuit = async (circuit: CompiledCircuit, inputs: any): Promise<{ witness: Uint8Array; returnValue: any }> => {
  const noir = new Noir(circuit as CompiledCircuit)

  return noir.execute(inputs)
}
