// ============================================================================
// SIMPLE SPINNER
// ============================================================================

import React from 'react'

interface SpinnerProps {
    size?: number
}

const Spinner: React.FC<SpinnerProps> = ({ size = 24 }) => (
    <div
        className={`animate-spin rounded-full border-2 border-enclave-400 border-t-transparent`}
        style={{ width: size, height: size }}
    />
)

export default Spinner