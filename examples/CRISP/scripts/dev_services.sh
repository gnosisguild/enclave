#!/usr/bin/env bash

set -euo pipefail

concurrently -kr \
  "./scripts/dev_cipher.sh" \
  "./scripts/dev_program.sh" \
  "sleep 3 && ./scripts/dev_server.sh" \
  "wait-on tcp:4000 && ./scripts/dev_client.sh"
