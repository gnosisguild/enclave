#!/usr/bin/env bash

set -euo pipefail

concurrently -r \
  "./scripts/tasks/dev_cipher.sh" \
  "sleep 3 && ./scripts/tasks/dev_add.sh" \
  "./scripts/tasks/dev_server.sh" \
  "./scripts/tasks/dev_client.sh"
