#!/usr/bin/env bash
# Ensure Noir circuit artifacts exist for a benchmark preset (insecure-512 | secure-8192).
#
# Usage (from repo root):
#   ./circuits/benchmarks/scripts/ensure_circuit_preset_built.sh <preset> [--force-build] [--verbose]
#
# Default: pnpm build:circuits --skip-if-built --no-clean --no-clean-targets (fast re-runs).
# --force-build: full rebuild (wipes dist/circuits and circuits/bin targets via build:circuits).

set -e

PRESET=""
FORCE_BUILD=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --force-build)
            FORCE_BUILD=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        -*)
            echo "Unknown option: $1"
            echo "Usage: $0 <preset> [--force-build] [--verbose]"
            exit 1
            ;;
        *)
            if [ -z "$PRESET" ]; then
                PRESET="$1"
            else
                echo "Unexpected argument: $1"
                exit 1
            fi
            shift
            ;;
    esac
done

if [ -z "$PRESET" ]; then
    echo "Usage: $0 <insecure-512|secure-8192> [--force-build] [--verbose]"
    exit 1
fi
if [ "$PRESET" != "insecure-512" ] && [ "$PRESET" != "secure-8192" ]; then
    echo "Error: preset must be insecure-512 or secure-8192"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

BUILD_ARGS=(--preset "$PRESET")
if [ "$FORCE_BUILD" = true ]; then
    echo "  [circuits] Full rebuild: pnpm build:circuits --preset ${PRESET}"
else
    BUILD_ARGS+=(--skip-if-built --no-clean --no-clean-targets)
    echo "  [circuits] Ensuring preset ${PRESET} (skip-if-built; use --force-build to recompile)..."
fi

if [ "$VERBOSE" = true ]; then
    echo "  [circuits] Running: pnpm build:circuits ${BUILD_ARGS[*]}"
    (cd "$REPO_ROOT" && pnpm build:circuits "${BUILD_ARGS[@]}")
else
    (cd "$REPO_ROOT" && pnpm build:circuits "${BUILD_ARGS[@]}" >/dev/null)
fi
