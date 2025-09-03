#!/usr/bin/env bash

set -e

if [ "$1" == "--ui" ]; then
  PLAYWRIGHT_CMD="pnpm synpress && pnpm playwright test"
else
  PLAYWRIGHT_CMD="pnpm synpress --headless && xvfb-run pnpm playwright test"
fi

concurrently -krs first "./scripts/setup.sh && ./scripts/dev.sh" "wait-on http://localhost:3000 && ${PLAYWRIGHT_CMD} && sleep 3"
