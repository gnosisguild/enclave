// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import * as WasmInstance from './crisp_wasm_crypto';

let wasmInstance = null;
let encryptInstance = null;

async function initWasm() {
    if (!wasmInstance) {
        wasmInstance = await WasmInstance.default();
        encryptInstance = new WasmInstance.Encrypt();
    }
}
initWasm();

self.onmessage = async function (event) {
    const { type, data } = event.data;
    switch (type) {
        case 'encrypt_vote':
            try {
                const { voteId, publicKey } = data;
                if (!wasmInstance || !encryptInstance) {
                    await initWasm();
                }
                const result = encryptInstance.encrypt_vote(voteId, publicKey);
                const circuitInputs = JSON.parse(result.circuit_inputs);

                self.postMessage({
                    type: 'encrypt_vote',
                    success: true,
                    encryptedVote: {
                        vote: result.encrypted_vote,
                        circuitInputs,
                    },
                });
            } catch (error) {
                self.postMessage({ type: 'encrypt_vote', success: false, error: error.message });
            }
            break;

        default:
            console.error(`Unknown message type: ${type}`);
    }
};
