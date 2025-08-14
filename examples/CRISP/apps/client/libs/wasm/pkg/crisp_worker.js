// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { EnclaveSDK } from '@gnosis-guild/enclave-sdk';
import circuit from "../../noir/crisp_circuit.json";

self.onmessage = async function (event) {
    const { type, data } = event.data;
    switch (type) {
        case 'encrypt_vote':
            try {
                const { voteId, publicKey } = data;
                const sdk = new EnclaveSDK({
                    
                })
                const result = await sdk.encryptNumberAndGenProof(
                    voteId,
                    publicKey,
                    circuit
                );
                
                self.postMessage({
                    type: 'encrypt_vote',
                    success: true,
                    encryptedVote: {
                        vote: result.encryptedVote,
                        proof: result.proof,
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
