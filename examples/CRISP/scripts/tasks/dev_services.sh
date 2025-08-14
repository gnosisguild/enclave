#!/usr/bin/env bash

set -euo pipefail

concurrently -kr \
  "./scripts/tasks/dev_cipher.sh" \
  "./scripts/tasks/dev_program.sh" \
  "sleep 3 && ./scripts/tasks/dev_server.sh" \
  "wait-on tcp:4000 && ./scripts/tasks/dev_client.sh"
