import {Identity} from "@semaphore-protocol/identity";
import {handleGenericError} from "@/utils/handle-generic-error.ts";
import {ethers} from "ethers";

// declare global {
//     interface Window {
//         ethereum?: any;
//     }
// }

//check eth provider in window
function hasEthereum(): boolean {
    return 'ethereum' in window;
}
export const createIdentityFromWallet = async (roundId: number): Promise<Identity> => {
    if (!hasEthereum()) {
        handleGenericError('createIdentityFromWallet', { name: 'WalletError', message: 'Wallet not connected' })
        throw new Error('Wallet not connected')
    }

    const provider = new ethers.BrowserProvider(window.ethereum)
    await provider.send("eth_requestAccounts", [])
    const signer = await provider.getSigner()

    const message = `${roundId}`
    const signature = await signer.signMessage(message)

    return new Identity(signature)
}