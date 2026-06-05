#!/usr/bin/env bash
# Shared paths and optional monorepo circuit build helpers for the default template.

_template_dev_config_root() {
  (cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
}

load_template_dev_config() {
  TEMPLATE_ROOT="$(_template_dev_config_root)"
  ENCLAVE_REPO_ROOT="$(cd "${TEMPLATE_ROOT}/../.." && pwd)"

  BFV_PRESET="${BFV_PRESET:-insecure-512}"
  COMMITTEE="${COMMITTEE:-micro}"

  case "$BFV_PRESET" in
    insecure-512 | secure-8192) ;;
    *)
      echo "Invalid BFV_PRESET='${BFV_PRESET}' (use insecure-512 or secure-8192)" >&2
      exit 1
      ;;
  esac

  export TEMPLATE_ROOT ENCLAVE_REPO_ROOT BFV_PRESET COMMITTEE
}

template_monorepo_build_available() {
  [[ -f "${ENCLAVE_REPO_ROOT}/scripts/build-circuits.ts" ]]
}

build_enclave_circuits_at_setup() {
  if ! template_monorepo_build_available; then
    echo "Skipping circuit build (standalone template; use enclave noir setup release artifacts)."
    return 0
  fi

  echo "Building enclave circuits (preset=${BFV_PRESET}, committee=${COMMITTEE})..."
  (
    cd "${ENCLAVE_REPO_ROOT}" &&
      pnpm build:circuits \
        --preset "${BFV_PRESET}" \
        --committee "${COMMITTEE}" \
        --skip-if-built
  )
}

sync_enclave_circuit_artifacts() {
  if ! template_monorepo_build_available; then
    return 0
  fi

  local src="${ENCLAVE_REPO_ROOT}/dist/circuits/${BFV_PRESET}/${COMMITTEE}"
  local dst="${TEMPLATE_ROOT}/.enclave/noir/circuits/${BFV_PRESET}/${COMMITTEE}"

  if [[ ! -f "${src}/recursive/dkg/pk/pk.json" ]]; then
    echo "No built circuits at ${src}; run pnpm dev:setup first. Using enclave noir setup release layout."
    return 0
  fi

  echo "Syncing circuits ${BFV_PRESET}/${COMMITTEE} → ${dst}"
  mkdir -p "$(dirname "${dst}")"
  rm -rf "${dst}"
  cp -R "${src}" "$(dirname "${dst}")/"
}
