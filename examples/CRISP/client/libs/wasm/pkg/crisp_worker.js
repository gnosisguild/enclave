import * as WasmInstance from './crisp_web';

let wasmInstance = null;
let encryptInstance = null;

async function initWasm() {
    if (!wasmInstance) {
        wasmInstance = await WasmInstance.default();
        console.log("Hardware Concurrency:", navigator.hardwareConcurrency);
        const maxThreads = 4;
        const numThreads = Math.max(
            navigator.hardwareConcurrency || 1,
            maxThreads
        );
        console.log("Number of Threads:", numThreads);

        try {
            await wasmInstance.initThreadPool(numThreads);
        } catch (error) {
            console.warn('Thread pool initialization failed:', error);
        }
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
                self.postMessage({
                    type: 'encrypt_vote',
                    success: true,
                    encryptedVote: result.encrypted_vote,
                    proof: result.proof,
                    instances: result.instances
                });
            } catch (error) {
                self.postMessage({ type: 'encrypt_vote', success: false, error: error.message });
            }
            break;

        default:
            console.error(`Unknown message type: ${type}`);
    }
};
