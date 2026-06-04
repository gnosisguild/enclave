#!/usr/bin/env bash
# Overlay per-committee circuit artifacts for CRISP ciphernodes.
#
# `enclave noir setup` installs the legacy release layout (`{preset}/recursive/...`).
# Runtime provers resolve `{preset}/{committee}/recursive/...` when present; this script
# builds or hydrates that tree into the CRISP `.enclave` dir.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/lib/dev_config.sh"

load_crisp_dev_config

COMMITTEE="${CRISP_COMMITTEE:-micro}"
CIRCUITS_OUT="${CRISP_ROOT}/.enclave/noir/circuits"
BB_BIN="${CRISP_ROOT}/.enclave/noir/bin/bb"

if [[ -x "${BB_BIN}" ]]; then
  export PATH="${CRISP_ROOT}/.enclave/noir/bin:${PATH}"
fi

echo "Staging enclave circuits (${CRISP_BFV_PRESET}/${COMMITTEE}) → ${CIRCUITS_OUT}"
mkdir -p "${CIRCUITS_OUT}"

(
  cd "${REPO_ROOT}"
  pnpm build:circuits \
    --preset "${CRISP_BFV_PRESET}" \
    --committee "${COMMITTEE}" \
    --skip-if-built \
    -o "${CIRCUITS_OUT}"
)
