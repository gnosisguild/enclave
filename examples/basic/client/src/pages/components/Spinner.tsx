// ============================================================================
// SIMPLE SPINNER
// ============================================================================

const Spinner: React.FC<{ size?: number; className?: string }> = ({ size = 18, className = "" }) => (
    <div className={`inline-block ${className}`}>
        <div
            className={`animate-spin rounded-full border-2 border-lime-400 border-t-transparent`}
            style={{ width: `${size}px`, height: `${size}px` }}
        />
    </div>
)

export default Spinner