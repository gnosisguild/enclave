#!/bin/bash
# Regenerate report.md from saved raw/*.json + crisp_verify_gas.json (no nargo, no integration re-run).
# Usage:
#   ./regenerate_report.sh
#   ./regenerate_report.sh --mode insecure
#   MODE=secure ./regenerate_report.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCHMARKS_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(cd "${BENCHMARKS_DIR}/../.." && pwd)"

MODE="${MODE:-secure}"
while [[ $# -gt 0 ]]; do
    case $1 in
        --mode)
            MODE="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--mode insecure|secure]"
            exit 1
            ;;
    esac
done

if [ "$MODE" != "insecure" ] && [ "$MODE" != "secure" ]; then
    echo "Error: mode must be insecure or secure"
    exit 1
fi

OUTPUT_DIR="${BENCHMARKS_DIR}/results_${MODE}"
GIT_COMMIT=$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo "unknown")
GIT_BRANCH=$(git -C "$REPO_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GAS_JSON="${OUTPUT_DIR}/crisp_verify_gas.json"
INTEGRATION_JSON="${OUTPUT_DIR}/integration_summary.json"

GR=( "${SCRIPT_DIR}/generate_report.sh"
    --input-dir "${OUTPUT_DIR}/raw"
    --output "${OUTPUT_DIR}/report.md"
    --git-commit "$GIT_COMMIT"
    --git-branch "$GIT_BRANCH"
    --gas-json "$GAS_JSON"
)
if [ -f "$INTEGRATION_JSON" ]; then
    GR+=(--integration-summary "$INTEGRATION_JSON")
fi
"${GR[@]}"

echo "✓ Report: ${OUTPUT_DIR}/report.md"
