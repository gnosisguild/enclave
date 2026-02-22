#!/bin/bash

# run_benchmarks.sh - Main orchestration script for benchmarking circuits
# Usage: ./run_benchmarks.sh [--config <config_file>] [--mode insecure|secure] [--circuit <path>] [--skip-compile] [--clean]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCHMARKS_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_FILE="${BENCHMARKS_DIR}/config.json"
CLEAN_ARTIFACTS=false
MODE_OVERRIDE=""
SKIP_COMPILE=false
CIRCUIT_FILTER=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --config)
            CONFIG_FILE="$2"
            shift 2
            ;;
        --mode)
            MODE_OVERRIDE="$2"
            if [ "$MODE_OVERRIDE" != "insecure" ] && [ "$MODE_OVERRIDE" != "secure" ]; then
                echo "Error: Mode must be 'insecure' or 'secure'"
                exit 1
            fi
            shift 2
            ;;
        --circuit)
            CIRCUIT_FILTER="$2"
            shift 2
            ;;
        --skip-compile|--no-compile)
            SKIP_COMPILE=true
            shift
            ;;
        --clean)
            CLEAN_ARTIFACTS=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--config <config_file>] [--mode insecure|secure] [--circuit <path>] [--skip-compile] [--clean]"
            exit 1
            ;;
    esac
done

if [ ! -f "$CONFIG_FILE" ]; then
    echo "Error: Config file not found: $CONFIG_FILE"
    exit 1
fi

echo "╔════════════════════════════════════════════════╗"
echo "║       Enclave ZK Circuit Benchmark Suite       ║"
echo "╚════════════════════════════════════════════════╝"
echo ""

# Read configuration (circuits may be strings or {name, modes[]}; see config.json)
ALL_CIRCUITS=$(jq -r '.circuits[] | (if type == "string" then . else .name end)' "$CONFIG_FILE")
ORACLES=$(jq -r '.oracles[]' "$CONFIG_FILE")
OUTPUT_DIR_BASE=$(jq -r '.output_dir // "results"' "$CONFIG_FILE")
BIN_DIR=$(jq -r '.bin_dir // "../bin"' "$CONFIG_FILE")
MODE=$(jq -r '.mode // "insecure"' "$CONFIG_FILE")

# Restrict to one circuit if --circuit was given
if [ -n "$CIRCUIT_FILTER" ]; then
    CIRCUITS="$CIRCUIT_FILTER"
    if ! echo "$ALL_CIRCUITS" | grep -qx "$CIRCUIT_FILTER" 2>/dev/null; then
        echo "Note: --circuit $CIRCUIT_FILTER is not in config.json; running anyway if path exists."
    fi
else
    CIRCUITS="$ALL_CIRCUITS"
fi

# Override mode if provided via command line
if [ -n "$MODE_OVERRIDE" ]; then
    MODE="$MODE_OVERRIDE"
fi

# Validate mode
if [ "$MODE" != "insecure" ] && [ "$MODE" != "secure" ]; then
    echo "Error: Invalid mode '$MODE'. Must be 'insecure' or 'secure'"
    exit 1
fi

# Monorepo root (benchmarks live in circuits/benchmarks, so go up two levels)
REPO_ROOT="$(cd "${BENCHMARKS_DIR}/../.." && pwd)"
# Circuits live under circuits/bin (bin_dir is relative to benchmarks dir, e.g. ../bin)
CIRCUITS_BASE_DIR="$(cd "${BENCHMARKS_DIR}/${BIN_DIR}" && pwd)"

# Create mode-specific output directory
OUTPUT_DIR="${OUTPUT_DIR_BASE}_${MODE}"
mkdir -p "${BENCHMARKS_DIR}/${OUTPUT_DIR}/raw"

# For secure mode, patch lib to use secure configs (restored at end)
DEFAULT_MOD_NR="${REPO_ROOT}/circuits/lib/src/configs/default/mod.nr"
DEFAULT_MOD_BACKUP=""
if [ "$MODE" = "secure" ] && [ -f "$DEFAULT_MOD_NR" ]; then
    DEFAULT_MOD_BACKUP="${DEFAULT_MOD_NR}.benchmark_backup"
    cp "$DEFAULT_MOD_NR" "$DEFAULT_MOD_BACKUP"
    if sed --version 2>/dev/null | grep -q GNU; then
        sed -i 's|super::insecure::|super::secure::|g' "$DEFAULT_MOD_NR"
    else
        sed -i '' 's|super::insecure::|super::secure::|g' "$DEFAULT_MOD_NR"
    fi
    echo "  Patched lib configs to secure (will restore)"
fi

# Store git info
GIT_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")

echo "Configuration:"
echo "  Mode: $MODE"
if [ -n "$CIRCUIT_FILTER" ]; then
    echo "  Circuit: $CIRCUIT_FILTER (single)"
fi
if [ "$SKIP_COMPILE" = true ]; then
    echo "  Skip Compilation: Yes (using existing artifacts)"
fi
echo "  Git Branch: $GIT_BRANCH"
echo "  Git Commit: $GIT_COMMIT"
echo "  Circuits: $(echo $CIRCUITS | wc -w | tr -d ' ')"
echo "  Oracles: $(echo $ORACLES)"
echo "  Base Directory: $CIRCUITS_BASE_DIR"
echo "  Output Directory: ${OUTPUT_DIR}"
echo ""

# decrypted_shares_aggregation_mod is for insecure only (Q < 128bit); _bn is for secure (large Q)
# Circuit-specific modes come from config.json (e.g. "config" has "modes": ["secure"]); see circuits/benchmarks/config.json
RUN_CIRCUITS=""
CIRCUIT_MODES=$(jq -r '.circuits[] | (if type == "string" then . else .name end) as $path | (if type == "object" and (.modes != null) then (.modes | join(",")) else "insecure,secure" end) | "\($path)\t\(.)"' "$CONFIG_FILE")
while IFS= read -r line; do
    [ -z "$line" ] && continue
    c="${line%%	*}"
    modes="${line#*	}"
    # If --circuit filter is set, we iterate over CIRCUITS (one path) and may not have entry in CIRCUIT_MODES; then run it
    if [ -n "$CIRCUIT_FILTER" ]; then
        [ "$c" != "$CIRCUIT_FILTER" ] && continue
    fi
    # Skip if this circuit is restricted to other mode(s) (see config.json "modes" field)
    if [ -n "$modes" ] && [ "${modes}" != "insecure,secure" ]; then
        if [[ ",${modes}," != *",${MODE},"* ]]; then
            echo "  Skipping $c (config.json restricts to mode(s): $modes; current mode: $MODE)"
            continue
        fi
    fi
    if [ "$MODE" = "secure" ] && [ "$c" = "threshold/decrypted_shares_aggregation_mod" ]; then
        echo "  Skipping $c (modular variant is insecure-only, Q < 128bit)"
        continue
    fi
    if [ "$MODE" = "insecure" ] && [ "$c" = "threshold/decrypted_shares_aggregation_bn" ]; then
        echo "  Skipping $c (BigNum variant is for secure/large Q only)"
        continue
    fi
    RUN_CIRCUITS="${RUN_CIRCUITS} ${c}"
done <<< "$CIRCUIT_MODES"
# When --circuit was given but not in config.json, no line matched; run it anyway if path exists (see note above)
if [ -n "$CIRCUIT_FILTER" ] && [ -z "$RUN_CIRCUITS" ] && ! echo "$ALL_CIRCUITS" | grep -qx "$CIRCUIT_FILTER" 2>/dev/null; then
    RUN_CIRCUITS="$CIRCUIT_FILTER"
fi
RUN_CIRCUITS=$(echo "$RUN_CIRCUITS" | xargs)
echo ""

TOTAL_BENCHMARKS=$(($(echo $RUN_CIRCUITS | wc -w | tr -d ' ') * $(echo $ORACLES | wc -w | tr -d ' ')))
CURRENT=0

# Restore lib config on exit (if we patched for secure)
restore_default_mod() {
    if [ -n "$DEFAULT_MOD_BACKUP" ] && [ -f "$DEFAULT_MOD_BACKUP" ]; then
        cp "$DEFAULT_MOD_BACKUP" "$DEFAULT_MOD_NR"
        rm -f "$DEFAULT_MOD_BACKUP"
        echo "  Restored lib configs/default to insecure"
    fi
}
trap restore_default_mod EXIT

# Run benchmarks
for CIRCUIT in $RUN_CIRCUITS; do
    CIRCUIT_PATH="${CIRCUITS_BASE_DIR}/${CIRCUIT}"
    
    if [ ! -d "$CIRCUIT_PATH" ]; then
        echo "⚠️  Warning: Circuit directory not found: $CIRCUIT_PATH"
        echo "    Skipping..."
        echo ""
        continue
    fi
    
    for ORACLE in $ORACLES; do
        CURRENT=$((CURRENT + 1))
        CIRCUIT_SLUG="$(echo "$CIRCUIT" | tr '/' '_')"
        OUTPUT_FILE="${BENCHMARKS_DIR}/${OUTPUT_DIR}/raw/${CIRCUIT_SLUG}_${ORACLE}.json"
        
        echo "────────────────────────────────────────────────"
        echo "Benchmark [$CURRENT/$TOTAL_BENCHMARKS]: ${CIRCUIT} (${MODE}) with ${ORACLE} oracle"
        echo "────────────────────────────────────────────────"
        
        # Generate Prover.toml (and configs.nr) via zk_cli so nargo execute has witness
        echo "  Generating Prover.toml..."
        if ! "${SCRIPT_DIR}/generate_prover_toml.sh" "$CIRCUIT" "$MODE" "$REPO_ROOT" 2>&1; then
            echo "⚠️  Prover.toml generation failed for $CIRCUIT, skipping benchmark"
            echo ""
            continue
        fi
        
        # Run benchmark
        BENCHMARK_ARGS=("$CIRCUIT_PATH" "$ORACLE" "$OUTPUT_FILE" "$MODE")
        if [ "$SKIP_COMPILE" = true ]; then
            BENCHMARK_ARGS+=("--skip-compile")
        fi
        "${SCRIPT_DIR}/benchmark_circuit.sh" "${BENCHMARK_ARGS[@]}"
        
        echo ""
    done
done

echo "╔════════════════════════════════════════════════╗"
echo "║       Generating Report...                     ║"
echo "╚════════════════════════════════════════════════╝"
echo ""

# Generate markdown report
REPORT_FILE="${BENCHMARKS_DIR}/${OUTPUT_DIR}/report.md"
"${SCRIPT_DIR}/generate_report.sh" \
    --input-dir "${BENCHMARKS_DIR}/${OUTPUT_DIR}/raw" \
    --output "${REPORT_FILE}" \
    --git-commit "$GIT_COMMIT" \
    --git-branch "$GIT_BRANCH"

echo "✓ Report generated: ${REPORT_FILE}"
echo ""

# Keep raw/ so that a later run with --circuit only overwrites that circuit's JSON and the
# report is regenerated from the full set (existing + updated). Delete results_<mode>/raw
# manually if you want a clean slate for the next full run.

# Clean artifacts if requested
if [ "$CLEAN_ARTIFACTS" = true ]; then
    echo "Cleaning circuit artifacts..."
    for CIRCUIT in $RUN_CIRCUITS; do
        CIRCUIT_PATH="${CIRCUITS_BASE_DIR}/${CIRCUIT}"
        if [ -d "$CIRCUIT_PATH/target" ]; then
            rm -rf "$CIRCUIT_PATH/target"
            echo "  ✓ Cleaned: $CIRCUIT (${MODE})"
        else
            echo "  ⊘ No target dir: $CIRCUIT (${MODE})"
        fi
    done
    echo ""
fi

echo "╔════════════════════════════════════════════════╗"
echo "║       Benchmark Complete!                      ║"
echo "╚════════════════════════════════════════════════╝"
echo ""
echo "Results:"
echo "  Report: ${REPORT_FILE}"
echo ""
echo "To view the report:"
echo "  cat ${REPORT_FILE}"
echo "  # or"
echo "  open ${REPORT_FILE}  # (macOS)"
