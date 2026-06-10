#!/usr/bin/env bash
# Compile threshold user-data encryption circuits for the SDK.
# Default committee is micro (matches DEFAULT_E3_CONFIG.committeeSize = CommitteeSize.Micro).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
COMMITTEE="${CIRCUIT_COMMITTEE:-micro}"
case "${COMMITTEE}" in
  micro|small|medium) ;;
  *)
    echo "Error: CIRCUIT_COMMITTEE must be micro|small|medium (got: ${COMMITTEE})" >&2
    exit 1
    ;;
esac
exec pnpm -C "${REPO_ROOT}" build:circuits \
  --preset insecure-512 \
  --committee "${COMMITTEE}" \
  --group threshold \
  --circuit user_data_encryption \
  --circuit user_data_encryption_ct0 \
  --circuit user_data_encryption_ct1 \
  --skip-vk \
  --skip-checksums \
  --no-clean-targets
