#!/bin/bash

# generate_prover_toml.sh - Generates Prover.toml (and configs.nr) for a circuit via zk_cli
# Usage: ./generate_prover_toml.sh <circuit_path> <mode> <repo_root>
#   circuit_path: e.g. "dkg/pk" or "threshold/share_decryption"
#   mode: "insecure" or "secure"
#   repo_root: absolute path to repository root (where Cargo.toml and circuits/ live)

set -e

CIRCUIT_PATH="$1"
MODE="$2"
REPO_ROOT="$3"

if [ -z "$CIRCUIT_PATH" ] || [ -z "$MODE" ] || [ -z "$REPO_ROOT" ]; then
    echo "Usage: $0 <circuit_path> <mode> <repo_root>"
    echo "  circuit_path: e.g. dkg/pk, threshold/share_decryption"
    echo "  mode: insecure or secure"
    echo "  repo_root: absolute path to repo root"
    exit 1
fi

if [ "$MODE" != "insecure" ] && [ "$MODE" != "secure" ]; then
    echo "Error: mode must be 'insecure' or 'secure'"
    exit 1
fi

PRESET="insecure"
[ "$MODE" = "secure" ] && PRESET="secure"

OUTPUT_DIR="${REPO_ROOT}/circuits/bin/${CIRCUIT_PATH}"

# Map circuit path to zk_cli --circuit and optional --inputs
# DKG circuits that need --inputs: share-computation, share-encryption, share-decryption
get_zk_args() {
    local path="$1"
    case "$path" in
        dkg/pk)
            echo "pk"
            return
            ;;
        dkg/sk_share_computation)
            echo "share-computation secret-key"
            return
            ;;
        dkg/e_sm_share_computation)
            echo "share-computation smudging-noise"
            return
            ;;
        dkg/share_encryption)
            echo "share-encryption secret-key"
            return
            ;;
        dkg/share_decryption)
            echo "share-decryption secret-key"
            return
            ;;
        threshold/user_data_encryption)
            echo "user-data-encryption"
            return
            ;;
        threshold/pk_generation)
            echo "pk-generation"
            return
            ;;
        threshold/pk_aggregation)
            echo "pk-aggregation"
            return
            ;;
        threshold/share_decryption)
            echo "threshold-share-decryption"
            return
            ;;
        threshold/decrypted_shares_aggregation_bn|threshold/decrypted_shares_aggregation_mod)
            echo "decrypted-shares-aggregation"
            return
            ;;
        *)
            echo "Error: unknown circuit path: $path" >&2
            exit 1
            ;;
    esac
}

ZK_ARGS=($(get_zk_args "$CIRCUIT_PATH"))
ZK_CIRCUIT="${ZK_ARGS[0]}"
ZK_INPUTS="${ZK_ARGS[1]:-}"

cd "$REPO_ROOT"

CMD=(cargo run -p e3-zk-helpers --bin zk_cli -- --circuit "$ZK_CIRCUIT" --preset "$PRESET" --output "$OUTPUT_DIR" --toml --no-configs)
if [ -n "$ZK_INPUTS" ]; then
    CMD+=(--inputs "$ZK_INPUTS")
fi

echo "  Generating Prover.toml: zk_cli --circuit $ZK_CIRCUIT --preset $PRESET ${ZK_INPUTS:+--inputs $ZK_INPUTS}"
if ! "${CMD[@]}" 2>&1; then
    echo "Error: zk_cli failed for $CIRCUIT_PATH"
    exit 1
fi
