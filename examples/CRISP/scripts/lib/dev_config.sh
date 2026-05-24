#!/usr/bin/env bash
# Shared CRISP local dev configuration. Source from setup.sh / crisp_deploy.sh.

_crisp_dev_config_root() {
  (cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
}

load_crisp_dev_config() {
  CRISP_ROOT="$(_crisp_dev_config_root)"
  REPO_ROOT="$(cd "${CRISP_ROOT}/../.." && pwd)"

  local cfg="${CRISP_ROOT}/crisp.dev.env"
  if [[ ! -f "$cfg" ]]; then
    cp "${CRISP_ROOT}/crisp.dev.env.example" "$cfg"
    echo "Created ${cfg} from crisp.dev.env.example"
  fi

  set -a
  # shellcheck disable=SC1090
  source "$cfg"
  set +a

  CRISP_BFV_PRESET="${CRISP_BFV_PRESET:-insecure-512}"
  CRISP_PROOF_AGGREGATION_ENABLED="${CRISP_PROOF_AGGREGATION_ENABLED:-false}"

  case "$CRISP_BFV_PRESET" in
    insecure-512 | secure-8192) ;;
    *)
      echo "Invalid CRISP_BFV_PRESET='${CRISP_BFV_PRESET}' (use insecure-512 or secure-8192)" >&2
      exit 1
      ;;
  esac

  case "$CRISP_PROOF_AGGREGATION_ENABLED" in
    true | false) ;;
    *)
      echo "Invalid CRISP_PROOF_AGGREGATION_ENABLED='${CRISP_PROOF_AGGREGATION_ENABLED}' (use true or false)" >&2
      exit 1
      ;;
  esac

  if [[ "$CRISP_PROOF_AGGREGATION_ENABLED" == "true" ]]; then
    export ENABLE_ZK_VERIFICATION=true
  else
    unset ENABLE_ZK_VERIFICATION
  fi

  export CRISP_BFV_PRESET CRISP_PROOF_AGGREGATION_ENABLED CRISP_ROOT REPO_ROOT
}

_set_env_kv() {
  local file=$1 key=$2 value=$3
  if [[ -f "$file" ]] && grep -q "^${key}=" "$file"; then
    local tmp
    tmp="$(mktemp)"
    sed "s|^${key}=.*|${key}=${value}|" "$file" >"$tmp"
    mv "$tmp" "$file"
  else
    echo "${key}=${value}" >>"$file"
  fi
}

apply_crisp_dev_config_to_server_env() {
  local server_env="${CRISP_ROOT}/server/.env"
  if [[ ! -f "$server_env" ]]; then
    cp "${CRISP_ROOT}/server/.env.example" "$server_env"
  fi
  _set_env_kv "$server_env" "E3_PROOF_AGGREGATION_ENABLED" "$CRISP_PROOF_AGGREGATION_ENABLED"
}

compile_enclave_dkg_circuits_if_needed() {
  if [[ "$CRISP_PROOF_AGGREGATION_ENABLED" != "true" ]]; then
    echo "Skipping enclave DKG circuit build (CRISP_PROOF_AGGREGATION_ENABLED=false in crisp.dev.env)"
    return 0
  fi

  echo "Building enclave DKG circuits (preset=${CRISP_BFV_PRESET})..."
  (
    cd "${REPO_ROOT}" &&
      pnpm build:circuits --preset "${CRISP_BFV_PRESET}" --skip-if-built
  )
}

print_crisp_dev_config_summary() {
  cat <<EOF

CRISP dev profile (${CRISP_ROOT}/crisp.dev.env):
  CRISP_BFV_PRESET=${CRISP_BFV_PRESET}
  CRISP_PROOF_AGGREGATION_ENABLED=${CRISP_PROOF_AGGREGATION_ENABLED}
  ENABLE_ZK_VERIFICATION=${ENABLE_ZK_VERIFICATION:-false} (used at deploy via dev:up)
  server/.env E3_PROOF_AGGREGATION_ENABLED synced by dev:setup
  Contract addresses synced by dev:up (deploy → server/.env, client/.env, enclave.config.yaml)

EOF
}

