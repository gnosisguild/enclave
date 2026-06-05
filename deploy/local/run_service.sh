#!/usr/bin/env bash
# =============================================================================
# CRISP Service Runner — called by VS Code tasks.
# Handles dependency waiting and launches one service.
#
# Usage: bash deploy/local/run_service.sh <service>
#   anvil      → start anvil
#   deploy     → deploy contracts + setup wallets/noir/ciphernodes + daemon
#   cn1..cn5   → start an individual ciphernode (waits for /tmp/crisp-dev-daemon-ready)
#   program    → start program server (waits for anvil :8545)
#   server     → start CRISP server (waits for program :13151)
#   client     → start CRISP client (waits for server :4000 + signal file)
# =============================================================================
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRISP_ROOT="$REPO_ROOT/examples/CRISP"
SIGNAL_FILE="/tmp/crisp-dev-daemon-ready"

SERVICE="${1:-}"

die() { echo "[run_service] ERROR: $*" >&2; exit 1; }

case "$SERVICE" in
  anvil)
    exec anvil \
      --host 0.0.0.0 \
      --chain-id 31337 \
      --block-time 1 \
      --mnemonic "test test test test test test test test test test test junk"
    ;;

  deploy)
    exec bash "${REPO_ROOT}/deploy/local/setup_nodes.sh"
    ;;

  cn1|cn2|cn3|cn4|cn5)
    echo "[run_service] Waiting for deploy to finish..."
    cd "${CRISP_ROOT}"
    while [ ! -f "${SIGNAL_FILE}" ]; do sleep 1; done
    echo "[run_service] Deploy done. Starting ${SERVICE} directly..."
    exec enclave --name "${SERVICE}" start -v
    ;;

  program)
    echo "[run_service] Waiting for Anvil (port 8545)..."
    while ! curl -s http://localhost:8545 >/dev/null 2>&1; do sleep 1; done
    echo "[run_service] Anvil ready. Starting program server..."
    cd "${CRISP_ROOT}"
    exec bash scripts/dev_program.sh
    ;;

  server)
    echo "[run_service] Waiting for Program Server (port 13151)..."
    while ! curl -s http://localhost:13151 >/dev/null 2>&1; do sleep 1; done
    echo "[run_service] Program server ready. Starting server..."
    cd "${CRISP_ROOT}"
    exec bash scripts/dev_server.sh
    ;;

  client)
    echo "[run_service] Waiting for Server (port 4000) and deploy signal..."
    while ! curl -s http://localhost:4000 >/dev/null 2>&1 || [ ! -f "${SIGNAL_FILE}" ]; do
      sleep 1
    done
    echo "[run_service] Ready. Starting client..."
    cd "${CRISP_ROOT}"
    exec bash scripts/dev_client.sh
    ;;

  *)
    die "Unknown service: '${SERVICE}'. Valid: anvil, deploy, cn1..cn5, program, server, client"
    ;;
esac
