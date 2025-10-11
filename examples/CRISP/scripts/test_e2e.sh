#!/usr/bin/env bash

set -e

if [ "$1" == "--ui" ]; then
  PLAYWRIGHT_CMD="pnpm synpress && pnpm playwright test"
else
  # Use xvfb-run only on Linux systems
  if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    PLAYWRIGHT_CMD="pnpm synpress --headless && xvfb-run pnpm playwright test"
  else
    PLAYWRIGHT_CMD="pnpm synpress --headless && pnpm playwright test"
  fi
fi

concurrently -krs first "./scripts/setup.sh && ./scripts/dev.sh" "wait-on http://localhost:3000 && ${PLAYWRIGHT_CMD} && sleep 3"
