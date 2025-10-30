#!/usr/bin/env bash

set -e

if [ "$1" == "--ui" ]; then
  PLAYWRIGHT_CMD="pnpm synpress && pnpm playwright test"
else
  # Use xvfb-run only on Linux systems
  if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    PLAYWRIGHT_CMD="pnpm synpress --headless && xvfb-run --auto-servernum --server-args=\"-screen 0 1280x960x24\" pnpm playwright test"
  else
    PLAYWRIGHT_CMD="pnpm synpress --headless && pnpm playwright test"
  fi
fi

echo "TEST E2E SCRIPT STARTING..."
pnpm concurrently -krs first "./scripts/setup.sh && ./scripts/dev.sh" "wait-on tcp:3000 && sleep 20 && ${PLAYWRIGHT_CMD} && sleep 3"
