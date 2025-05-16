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
                const encryptedVote = encryptInstance.encrypt_vote(voteId, publicKey);
                self.postMessage({ type: 'encrypt_vote', success: true, encryptedVote });
            } catch (error) {
                self.postMessage({ type: 'encrypt_vote', success: false, error: error.message });
            }
            break;

        default:
            console.error(`Unknown message type: ${type}`);
    }
};
