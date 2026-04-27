#!/bin/bash

# extract_crisp_verify_gas.sh - Runs CRISP verifier test with gas reporter and emits JSON.
# Usage: ./extract_crisp_verify_gas.sh --output <json_file>

set -e

OUTPUT_JSON=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --output)
            OUTPUT_JSON="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 --output <json_file>"
            exit 1
            ;;
    esac
done

if [ -z "$OUTPUT_JSON" ]; then
    echo "Usage: $0 --output <json_file>"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
CRISP_CONTRACTS_DIR="${REPO_ROOT}/examples/CRISP/packages/crisp-contracts"
TMP_LOG_CRISP="$(mktemp)"
TMP_LOG_FOLDED="$(mktemp)"
TMP_LOG_ENCLAVE="$(mktemp)"
TMP_JSON_ENCLAVE="$(mktemp)"
TMP_JSON_FOLDED="$(mktemp)"

cleanup_tmp_files() {
    rm -f "$TMP_LOG_CRISP" "$TMP_LOG_FOLDED" "$TMP_LOG_ENCLAVE" "$TMP_JSON_ENCLAVE" "$TMP_JSON_FOLDED"
}
trap cleanup_tmp_files EXIT

if [ ! -d "$CRISP_CONTRACTS_DIR" ]; then
    cat > "$OUTPUT_JSON" <<EOF
{
  "verify_gas": null,
  "source": "crisp_verify_test",
  "note": "CRISP contracts directory not found"
}
EOF
    exit 0
fi

ENCLAVE_CONTRACTS_DIR="${REPO_ROOT}/packages/enclave-contracts"
OUTPUT_DIR="$(cd "$(dirname "$OUTPUT_JSON")" && pwd)"
RAW_DIR="${OUTPUT_DIR}/raw"

# Ensure recursive/noir VK variants exist for integration-based folded-proof export.
# This populates target artifacts required by `test_trbfv_actor`.
(
  cd "$REPO_ROOT" && \
  pnpm build:circuits --preset insecure-512 >/dev/null
)

set +e
(
  cd "$CRISP_CONTRACTS_DIR" && \
  pnpm hardhat test mocha --grep "should verify the proof correctly with the crisp verifier"
) > "$TMP_LOG_CRISP" 2>&1
CRISP_TEST_EXIT_CODE=$?
(
  cd "$REPO_ROOT" && \
  BENCHMARK_FOLDED_OUTPUT="$TMP_JSON_FOLDED" cargo test -p e3-tests test_trbfv_actor -- --nocapture
) > "$TMP_LOG_FOLDED" 2>&1
FOLDED_TEST_EXIT_CODE=$?
(
  cd "$ENCLAVE_CONTRACTS_DIR" && \
  BENCHMARK_RAW_DIR="$RAW_DIR" BENCHMARK_GAS_OUTPUT="$TMP_JSON_ENCLAVE" BENCHMARK_FOLDED_JSON="$TMP_JSON_FOLDED" \
  pnpm hardhat run scripts/benchmarkGasFromRaw.ts --network hardhat
) > "$TMP_LOG_ENCLAVE" 2>&1
ENCLAVE_TEST_EXIT_CODE=$?
set -e

parse_marker() {
    local marker="$1"
    local file_path="$2"
    python3 - "$marker" "$file_path" <<'PY'
import re
import sys

marker = sys.argv[1]
path = sys.argv[2]
with open(path, "r", encoding="utf-8", errors="ignore") as f:
    text = f.read()

m = re.search(rf"\[bench-gas\]\s+{re.escape(marker)}=(\d+)", text)
if m:
    print(m.group(1))
PY
}

USER_VERIFY_GAS=$(parse_marker "crisp_user_verify" "$TMP_LOG_CRISP")
DKG_VERIFY_GAS=$(jq -r '.verify_gas.dkg // empty' "$TMP_JSON_ENCLAVE" 2>/dev/null || true)
DEC_VERIFY_GAS=$(jq -r '.verify_gas.dec // empty' "$TMP_JSON_ENCLAVE" 2>/dev/null || true)

[ -z "$USER_VERIFY_GAS" ] && USER_VERIFY_GAS="null"
[ -z "$DKG_VERIFY_GAS" ] && DKG_VERIFY_GAS="null"
[ -z "$DEC_VERIFY_GAS" ] && DEC_VERIFY_GAS="null"

cat > "$OUTPUT_JSON" <<EOF
{
  "verify_gas": {
    "dkg": ${DKG_VERIFY_GAS},
    "user": ${USER_VERIFY_GAS},
    "dec": ${DEC_VERIFY_GAS}
  },
  "source": "folded_proof_export_plus_crisp_verify_test",
  "test_exit_code": {
    "crisp": ${CRISP_TEST_EXIT_CODE},
    "folded_export": ${FOLDED_TEST_EXIT_CODE},
    "enclave_contracts": ${ENCLAVE_TEST_EXIT_CODE}
  }
}
EOF
