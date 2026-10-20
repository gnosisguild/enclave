#!/usr/bin/env bash
# Time one client-side CRISP vote proof for a preset, on native bb and on WASM.
#
# Usage: scripts/benchmark-vote-proof.sh [preset] [runs]
#   e.g. scripts/benchmark-vote-proof.sh secure-8192 2
#
# It builds the zk-inputs wasm, compiles and stages the preset's circuits with
# scripts/build-presets.mjs, then runs packages/crisp-sdk/tests/vote-proof-benchmark.test.ts once
# with native bb and once on WASM. On exit it rebuilds the root circuits that were active before
# (from dist/circuits when that archive is current) and puts back the CRISP circuit targets and
# generated verifiers it replaced.
#
# Needs the pinned bb first on PATH (`required_bb_version` in crates/zk-prover/versions.json).
# BB_NATIVE overrides the binary the native run uses. CRISP_BENCH_SKIP_BUILD=1 reuses the staged
# circuits/dist/<preset>/ when check-staged-preset.mjs accepts it. Logs go to a temporary directory.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRISP="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO="$(cd "${CRISP}/../.." && pwd)"
PRESET="${1:-secure-8192}"
RUNS="${2:-2}"
SDK="${CRISP}/packages/crisp-sdk"
LOGS="$(mktemp -d "${TMPDIR:-/tmp}/crisp-vote-bench.XXXXXX")"

case "${PRESET}" in
  insecure-64 | secure-8192) ;;
  *)
    echo "Unknown preset: ${PRESET} (insecure-64 or secure-8192)" >&2
    exit 1
    ;;
esac

PINNED_BB="$(node -e 'console.log(require(process.argv[1]).required_bb_version)' "${REPO}/crates/zk-prover/versions.json")"
NATIVE="${BB_NATIVE:-$(command -v bb || true)}"
if [[ -z "${NATIVE}" || "$("${NATIVE}" --version)" != "${PINNED_BB}" || "$(bb --version)" != "${PINNED_BB}" ]]; then
  echo "Put bb ${PINNED_BB} first on PATH (found ${NATIVE:-none}); the circuit build and the native run need it." >&2
  exit 1
fi
# The root builder compiles e3-zk-artifacts for the parity matrices; it does not need the zk-prover fixtures.
export E3_ZK_PROVER_SKIP_FIXTURE_BUILD="${E3_ZK_PROVER_SKIP_FIXTURE_BUILD:-1}"

ACTIVE="${REPO}/circuits/bin/.active-preset.json"
read -r ACTIVE_PRESET ACTIVE_COMMITTEE < <(node -e '
  const a = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"))
  console.log(a.preset, a.committee)' "${ACTIVE}")

# build-presets.mjs recompiles these for the preset, and the root builder does not know about them.
SAVED=(
  circuits/bin/crisp/target
  circuits/bin/crisp_onchain/target
  circuits/bin/fold/target
  circuits/bin/fold_onchain/target
  packages/crisp-contracts/contracts/verifiers/CRISPVerifier.sol
  packages/crisp-contracts/contracts/verifiers/CRISPOnchainVerifier.sol
)
BACKUP="${LOGS}/saved"

restore() {
  local status=$?
  if [[ -d "${BACKUP}" ]]; then
    echo "Restoring the ${ACTIVE_PRESET}/${ACTIVE_COMMITTEE} circuits"
    pnpm -C "${REPO}" build:circuits --preset "${ACTIVE_PRESET}" --committee "${ACTIVE_COMMITTEE}" --skip-if-built \
      > "${LOGS}/restore.log" 2>&1 || { echo "Restore failed; see ${LOGS}/restore.log" >&2; status=1; }
    for path in "${SAVED[@]}"; do
      if [[ -e "${BACKUP}/${path}" ]]; then
        rm -rf "${CRISP:?}/${path}"
        cp -Rp "${BACKUP}/${path}" "${CRISP}/${path}"
      fi
    done
  fi
  echo "Logs: ${LOGS}"
  exit "${status}"
}
trap restore EXIT

if [[ "${CRISP_BENCH_SKIP_BUILD:-0}" == 1 ]]; then
  node "${CRISP}/scripts/check-staged-preset.mjs" "${PRESET}"
else
  for path in "${SAVED[@]}"; do
    if [[ -e "${CRISP}/${path}" ]]; then
      mkdir -p "$(dirname "${BACKUP}/${path}")"
      cp -Rp "${CRISP}/${path}" "${BACKUP}/${path}"
    fi
  done
  mkdir -p "${BACKUP}"

  echo "Building the zk-inputs wasm"
  pnpm -C "${CRISP}/packages/crisp-zk-inputs" build > "${LOGS}/wasm.log" 2>&1 \
    || { echo "wasm build failed; see ${LOGS}/wasm.log" >&2; exit 1; }
  echo "Building and staging the ${PRESET} circuits"
  node "${CRISP}/scripts/build-presets.mjs" --preset "${PRESET}" > "${LOGS}/build.log" 2>&1 \
    || { echo "circuit build failed; see ${LOGS}/build.log" >&2; exit 1; }
fi

cd "${SDK}"
# The test asks bb.js for its native backend when BB_BINARY_PATH names a file, and otherwise keeps
# the SDK's WASM backend.
for bb in "${NATIVE}" /nonexistent/bb; do
  log="${LOGS}/$( [[ "${bb}" == /nonexistent/bb ]] && echo wasm || echo native ).log"
  echo "Proving with BB_BINARY_PATH=${bb}"
  if ! BB_BINARY_PATH="${bb}" CRISP_BENCH_PRESET="${PRESET}" CRISP_BENCH_RUNS="${RUNS}" \
    pnpm exec vitest --run tests/vote-proof-benchmark.test.ts > "${log}" 2>&1; then
    grep -E 'CRISP_BENCH|FAIL|Error' "${log}" || true
    echo "Benchmark failed; see ${log}" >&2
    exit 1
  fi
  grep -E 'CRISP_BENCH' "${log}"
done
