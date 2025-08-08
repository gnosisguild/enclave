// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { bfvVerifiableEncryptNumber } from '@gnosis-guild/enclave-sdk';

self.onmessage = async function (event) {
    const { type, data } = event.data;
    switch (type) {
        case 'encrypt_vote':
            try {
                const { voteId, publicKey } = data;
                const result = await bfvVerifiableEncryptNumber(
                    voteId,
                    publicKey
                );
                const circuitInputs = JSON.parse(result.circuitInputs);

                self.postMessage({
                    type: 'encrypt_vote',
                    success: true,
                    encryptedVote: {
                        vote: result.encryptedVote,
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
