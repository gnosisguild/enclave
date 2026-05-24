#!/bin/bash
# Shared output directory naming for benchmark runs.
# Sourced by run_benchmarks.sh and regenerate_report.sh.
#
# Four default layouts (under circuits/benchmarks/):
#   results_insecure_agg | results_insecure_no_agg
#   results_secure_agg   | results_secure_no_agg

# Args: <insecure|secure> <proof_aggregation: true|false|on|off|...>
benchmark_results_dir_basename() {
    local mode="$1"
    local proof_agg="${2:-true}"
    local base="${BENCHMARK_OUTPUT_DIR_BASE:-results}"
    local suffix="agg"
    case "$(echo "$proof_agg" | tr '[:upper:]' '[:lower:]')" in
        0|false|no|off) suffix="no_agg" ;;
    esac
    echo "${base}_${mode}_${suffix}"
}

# Full path under BENCHMARKS_DIR (set by caller).
benchmark_results_dir_path() {
    local benchmarks_dir="$1"
    local mode="$2"
    local proof_agg="${3:-true}"
    echo "${benchmarks_dir}/$(benchmark_results_dir_basename "$mode" "$proof_agg")"
}

# Legacy: results_<mode> (no agg suffix). Used as fallback when regenerating old runs.
benchmark_results_dir_legacy_basename() {
    local mode="$1"
    local base="${BENCHMARK_OUTPUT_DIR_BASE:-results}"
    echo "${base}_${mode}"
}
