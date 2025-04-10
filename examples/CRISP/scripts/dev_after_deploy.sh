#!/usr/bin/env bash

set -euo pipefail

concurrently -r \
  "./scripts/dev_cipher.sh" \
  "./scripts/dev_agg.sh" \
  "./scripts/dev_add.sh" \
  "./scripts/dev_server.sh" \
  "./scripts/dev_client.sh"
