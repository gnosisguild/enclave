#!/usr/bin/env bash

set -e

if [ "$1" == "--ui" ]; then
  PLAYWRIGHT_CMD="cd apps/client && pnpm synpress && pnpm playwright test --ui"
else
  PLAYWRIGHT_CMD="cd apps/client && pnpm synpress --headless && xvfb-run pnpm playwright test"
fi

concurrently -krs first "./scripts/setup.sh && ./scripts/dev.sh" "wait-on http://localhost:3000 && ${PLAYWRIGHT_CMD} && sleep 3"
