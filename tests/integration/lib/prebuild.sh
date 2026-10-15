#!/usr/bin/env bash
set -eu  # Exit immediately if a command exits with a non-zero status

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
INTEGRATION_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
INTEGRATION_NOIR="${INTEGRATION_DIR}/.interfold/noir"
VERSIONS_JSON="${ROOT_DIR}/crates/zk-prover/versions.json"

echo ""
echo "PREBUILDING BINARIES..."
echo ""
(cd "$ROOT_DIR/crates" && E3_ZK_PROVER_SKIP_FIXTURE_BUILD=1 cargo build --bin fake_encrypt --bin pack_e3_params)
echo ""
echo "FINISHED PREBUILDING BINARIES"
echo ""

echo ""
echo "BUILDING SOURCE-ALIGNED ZK CIRCUITS..."
echo ""

# The ciphernode always generates leaf proofs, even when recursive aggregation
# is skipped. Never let integration tests combine current Rust witnesses with
# a stale circuit ABI.
rm -rf "${INTEGRATION_NOIR}/circuits"
mkdir -p "${INTEGRATION_NOIR}/circuits" "${INTEGRATION_NOIR}/bin"

if [[ "${FULL_PROOF_AGGREGATION:-false}" == "true" ]]; then
  # Full-proof tests use the newest published matrix. Its SOURCE_HASH is not checked against this
  # tree until the circuit drift workflow is designed.
  (cd "$ROOT_DIR" && pnpm store:circuits pull --latest)
  (cd "$ROOT_DIR" && pnpm build:circuits \
    --preset insecure-512 \
    --committee minimum \
    --hydrate-bin-only \
    --no-clean-targets)
  for preset in insecure-512 secure-8192; do
    mkdir -p "${INTEGRATION_NOIR}/circuits/${preset}"
    cp -R "${ROOT_DIR}/dist/circuits/${preset}/minimum" \
      "${INTEGRATION_NOIR}/circuits/${preset}/"
  done
  # `--check`: verify the committed Honk Solidity verifiers in
  # packages/interfold-contracts/contracts/verifiers/bfv/honk/ match the
  # freshly-built circuits' recursive VKs. Fails loudly on drift instead of
  # silently rewriting committed contracts mid-test. If this errors, run
  # `pnpm generate:verifiers --write` and commit the diff.
  (cd "$ROOT_DIR" && pnpm generate:verifiers --check --no-compile --no-clean-targets)
else
  # C5/C7 final aggregation is skipped, but DKG and decryption leaf proofs
  # still execute. Build only those two source groups for the fast CI profile.
  (cd "$ROOT_DIR" && pnpm build:circuits --preset insecure-512 --group dkg,threshold -o "${INTEGRATION_NOIR}/circuits")
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required to pin noir/version.json for integration ZK fixtures" >&2
  exit 1
fi
if ! command -v bb >/dev/null 2>&1; then
  echo "bb is required to build source-aligned integration circuits" >&2
  exit 1
fi

REQUIRED_BB="$(jq -r '.required_bb_version' "$VERSIONS_JSON")"
REQUIRED_CIRCUITS="$(jq -r '.required_circuits_version' "$VERSIONS_JSON")"
source "$SCRIPT_DIR/utils.sh"
CIRCUITS_MAP="$(
  cd "${INTEGRATION_NOIR}/circuits"
  find . -type f \
    ! -path './checksums.json' \
    ! -path './SHA256SUMS' \
    ! -path './SOURCE_HASH' \
    ! -name '.build-stamp.json' \
    -print |
    LC_ALL=C sort |
    while IFS= read -r file; do
      relative="${file#./}"
      checksum="$(sha256_of_stdin <"$file")"
      jq -n --arg file "$relative" --arg checksum "$checksum" \
        '{key: $file, value: {file: $file, checksum: $checksum}}'
    done |
    jq -s 'map({(.key): .value}) | add // {}'
)"
if [[ "$(jq 'length' <<<"$CIRCUITS_MAP")" -eq 0 ]]; then
  echo "no circuit artifacts found for integration ZK fixtures" >&2
  exit 1
fi
jq -n \
  --arg bb "$REQUIRED_BB" \
  --arg circuits "$REQUIRED_CIRCUITS" \
  --argjson circuit_files "$CIRCUITS_MAP" \
  '{bb_version: $bb, circuits_version: $circuits, circuits: $circuit_files}' \
  > "${INTEGRATION_NOIR}/version.json"
cp "$(command -v bb)" "${INTEGRATION_NOIR}/bin/bb"
chmod +x "${INTEGRATION_NOIR}/bin/bb"

# The upgrade reset scenario keeps this output for the candidate. The stamp lets it refuse a noir
# folder that a released binary's setup replaced.
noir_digest "$INTEGRATION_NOIR" >"${INTEGRATION_DIR}/.interfold/prebuild-noir.sha256"

echo "Staged source-aligned integration circuits under ${INTEGRATION_NOIR}/circuits"
echo "Pinned noir version.json (bb=${REQUIRED_BB}, circuits=${REQUIRED_CIRCUITS})"
echo ""
echo "FINISHED BUILDING SOURCE-ALIGNED ZK CIRCUITS"
echo ""
