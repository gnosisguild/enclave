#!/bin/bash
# Shared output directory naming for benchmark runs.
# Sourced by run_benchmarks.sh and regenerate_report.sh.
#
# Layout (under circuits/benchmarks/) — three axes, hyphen-separated:
#   results_<mode>_<agg|no_agg>_<committee>
#
# Examples:
#   results_insecure_agg_micro
#   results_insecure_no_agg_micro
#   results_insecure_agg_medium
#   results_secure_agg_micro
#
# Three layouts (mode, proof aggregation, committee) so micro/small/medium runs coexist on
# disk for direct A/B comparison without manual renames.

# Args: <insecure|secure> <proof_aggregation: true|false|on|off|...> [committee]
benchmark_results_dir_basename() {
    local mode="$1"
    local proof_agg="${2:-true}"
    local committee="${3:-micro}"
    local base="${BENCHMARK_OUTPUT_DIR_BASE:-results}"
    local suffix="agg"
    case "$(echo "$proof_agg" | tr '[:upper:]' '[:lower:]')" in
        0|false|no|off) suffix="no_agg" ;;
    esac
    echo "${base}_${mode}_${suffix}_${committee}"
}

# Full path under BENCHMARKS_DIR (set by caller).
benchmark_results_dir_path() {
    local benchmarks_dir="$1"
    local mode="$2"
    local proof_agg="${3:-true}"
    local committee="${4:-micro}"
    echo "${benchmarks_dir}/$(benchmark_results_dir_basename "$mode" "$proof_agg" "$committee")"
}

# Legacy basenames the regenerator falls back to when no committee-aware dir exists:
#   1. results_<mode>_<suffix>   — committee-less layout (pre-committee axis)
#   2. results_<mode>             — pre-suffix layout (oldest)
# Returns each candidate on its own line; callers iterate and pick the first that exists.
benchmark_results_dir_legacy_basenames() {
    local mode="$1"
    local proof_agg="${2:-true}"
    local base="${BENCHMARK_OUTPUT_DIR_BASE:-results}"
    local suffix="agg"
    case "$(echo "$proof_agg" | tr '[:upper:]' '[:lower:]')" in
        0|false|no|off) suffix="no_agg" ;;
    esac
    echo "${base}_${mode}_${suffix}"
    echo "${base}_${mode}"
}

# Backward compat: single-string version (oldest legacy only). Kept so older external scripts
# that source this file don't break.
benchmark_results_dir_legacy_basename() {
    local mode="$1"
    local base="${BENCHMARK_OUTPUT_DIR_BASE:-results}"
    echo "${base}_${mode}"
}
