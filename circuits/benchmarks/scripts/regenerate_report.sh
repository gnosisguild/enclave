#!/bin/bash
# Regenerate report.md from saved raw/*.json + crisp_verify_gas.json (no nargo, no integration re-run).
# Usage:
#   ./regenerate_report.sh
#   ./regenerate_report.sh --mode insecure
#   ./regenerate_report.sh --mode insecure --no-proof-aggregation
#   MODE=secure ./regenerate_report.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCHMARKS_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(cd "${BENCHMARKS_DIR}/../.." && pwd)"
# shellcheck source=benchmark_output_dir.sh
source "${SCRIPT_DIR}/benchmark_output_dir.sh"

MODE="${MODE:-secure}"
PROOF_AGGREGATION="${BENCHMARK_PROOF_AGGREGATION:-true}"
while [[ $# -gt 0 ]]; do
    case $1 in
        --mode)
            MODE="$2"
            shift 2
            ;;
        --proof-aggregation)
            PROOF_AGGREGATION="$2"
            shift 2
            ;;
        --no-proof-aggregation)
            PROOF_AGGREGATION=false
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--mode insecure|secure] [--proof-aggregation on|off] [--no-proof-aggregation]"
            exit 1
            ;;
    esac
done

case "$(echo "$PROOF_AGGREGATION" | tr '[:upper:]' '[:lower:]')" in
    0|false|no|off) PROOF_AGGREGATION=false ;;
    *) PROOF_AGGREGATION=true ;;
esac

if [ "$MODE" != "insecure" ] && [ "$MODE" != "secure" ]; then
    echo "Error: mode must be insecure or secure"
    exit 1
fi

OUTPUT_DIR="$(benchmark_results_dir_path "$BENCHMARKS_DIR" "$MODE" "$PROOF_AGGREGATION")"
# Backward compatibility: pre-suffix layout results_<mode>/
if [ ! -d "${OUTPUT_DIR}/raw" ] && [ ! -f "${OUTPUT_DIR}/crisp_verify_gas.json" ]; then
    LEGACY="${BENCHMARKS_DIR}/$(benchmark_results_dir_legacy_basename "$MODE")"
    if [ -d "${LEGACY}/raw" ] || [ -f "${LEGACY}/crisp_verify_gas.json" ]; then
        echo "Note: using legacy output dir ${LEGACY} (rename to $(basename "$OUTPUT_DIR") to match new layout)"
        OUTPUT_DIR="$LEGACY"
    fi
fi
GIT_COMMIT=$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo "unknown")
GIT_BRANCH=$(git -C "$REPO_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
GAS_JSON="${OUTPUT_DIR}/crisp_verify_gas.json"
INTEGRATION_JSON="${OUTPUT_DIR}/integration_summary.json"

RUN_META="${OUTPUT_DIR}/benchmark_run_meta.json"
GR=( "${SCRIPT_DIR}/generate_report.sh"
    --input-dir "${OUTPUT_DIR}/raw"
    --output "${OUTPUT_DIR}/report.md"
    --git-commit "$GIT_COMMIT"
    --git-branch "$GIT_BRANCH"
    --gas-json "$GAS_JSON"
    --benchmark-mode "$MODE"
)
if [ -f "$RUN_META" ]; then
    GR+=(--run-meta "$RUN_META")
fi
if [ -f "$INTEGRATION_JSON" ]; then
    GR+=(--integration-summary "$INTEGRATION_JSON")
fi
"${GR[@]}"

echo "✓ Report: ${OUTPUT_DIR}/report.md"
