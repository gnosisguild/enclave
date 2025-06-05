/**
 * Format contract errors into user-friendly messages
 */
export function formatContractError(error: any): string {
    if (!error) return 'Unknown error occurred';

    if (error.name === 'ContractFunctionExecutionError') {
        const functionMatch = error.message.match(/The contract function "([^"]+)" reverted/);
        const functionName = functionMatch ? functionMatch[1] : 'contract function';

        const contractErrors: Record<string, string> = {
            'publishInput': 'Failed to submit encrypted inputs. The computation may not be ready or inputs are invalid.',
            'activate': 'Failed to activate the computation environment. Please ensure the committee has been published.',
            'request': 'Failed to request computation. Please check your parameters and try again.',
            'addCiphernode': 'Failed to add ciphernode. You may not have permission or the node is already registered.',
            'removeCiphernode': 'Failed to remove ciphernode. You may not have permission or invalid parameters.'
        };

        return contractErrors[functionName] || `The ${functionName} operation failed. Please check your inputs and try again.`;
    }

    if (error.message) {
        if (error.message.includes('User rejected')) {
            return 'Transaction was cancelled by user.';
        }
        if (error.message.includes('insufficient funds')) {
            return 'Insufficient funds to complete the transaction.';
        }
        if (error.message.includes('nonce too low')) {
            return 'Transaction nonce error. Please refresh and try again.';
        }
        if (error.message.includes('gas')) {
            return 'Transaction failed due to gas issues. Please try again with higher gas.';
        }
        if (error.message.includes('network')) {
            return 'Network error occurred. Please check your connection and try again.';
        }
    }

    if (error.code && error.message) {
        return error.message;
    }

    return 'An unexpected error occurred. Please try again.';
}

/**
 * Extract a simple error message for display
 */
export function getDisplayErrorMessage(error: any): string {
    const formatted = formatContractError(error);
    return formatted;
} 