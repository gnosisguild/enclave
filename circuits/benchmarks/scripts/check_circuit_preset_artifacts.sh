#!/usr/bin/env bash
# Verify dist + circuits/bin artifacts exist for a benchmark preset.
# Exit 0 if ready, 1 if not (prints missing paths on stderr).
#
# Usage: ./check_circuit_preset_artifacts.sh <insecure-512|secure-8192>

set -e

PRESET="${1:-}"
if [ -z "$PRESET" ]; then
    echo "Usage: $0 <insecure-512|secure-8192>" >&2
    exit 1
fi
if [ "$PRESET" != "insecure-512" ] && [ "$PRESET" != "secure-8192" ]; then
    echo "Error: preset must be insecure-512 or secure-8192" >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
DIST="${REPO_ROOT}/dist/circuits/${PRESET}"
BIN="${REPO_ROOT}/circuits/bin"
STAMP="${DIST}/.build-stamp.json"

MARKERS=(
    "${DIST}/default/recursive_aggregation/dkg_aggregator/dkg_aggregator.json"
    "${DIST}/default/recursive_aggregation/decryption_aggregator/decryption_aggregator.json"
    "${BIN}/recursive_aggregation/dkg_aggregator/target/dkg_aggregator.json"
    "${BIN}/recursive_aggregation/dkg_aggregator/target/dkg_aggregator.vk_recursive"
    "${BIN}/recursive_aggregation/decryption_aggregator/target/decryption_aggregator.json"
    "${BIN}/recursive_aggregation/decryption_aggregator/target/decryption_aggregator.vk_recursive"
    "${BIN}/dkg/target/pk.json"
    "${BIN}/threshold/target/pk_aggregation.json"
)

missing=()
for path in "${MARKERS[@]}"; do
    if [ ! -f "$path" ]; then
        missing+=("$path")
    fi
done

ACTIVE="${BIN}/.active-preset.json"

if [ ! -f "$STAMP" ]; then
    missing+=("$STAMP")
elif ! jq -e --arg p "$PRESET" '.preset == $p' "$STAMP" >/dev/null 2>&1; then
    echo "Error: ${STAMP} is for a different preset (expected ${PRESET})." >&2
    echo "  Run: pnpm build:circuits --preset ${PRESET}" >&2
    exit 1
fi

if [ ! -f "$ACTIVE" ]; then
    missing+=("$ACTIVE")
elif ! jq -e --arg p "$PRESET" '.preset == $p' "$ACTIVE" >/dev/null 2>&1; then
    echo "Error: circuits/bin was last built for a different preset (see ${ACTIVE})." >&2
    echo "  Fast fix (no full recompile if dist is ready):" >&2
    echo "    pnpm build:circuits --preset ${PRESET} --skip-if-built --no-clean --no-clean-targets" >&2
    exit 1
fi

if [ ${#missing[@]} -gt 0 ]; then
    echo "Error: circuit artifacts for preset '${PRESET}' are missing or stale." >&2
    echo "  circuits/bin/target reflects the last preset built; dist/circuits/<preset>/ must exist for this mode." >&2
    echo "  Fix: pnpm build:circuits --preset ${PRESET}" >&2
    echo "  Or run this script without --skip-build." >&2
    echo "Missing:" >&2
    printf '  %s\n' "${missing[@]}" >&2
    exit 1
fi

exit 0
