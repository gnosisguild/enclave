#!/usr/bin/env bash

set -e

if [ "$1" == "--ui" ]; then
  PLAYWRIGHT_CMD="pnpm synpress && pnpm playwright test --ui"
else
  PLAYWRIGHT_CMD="pnpm synpress --headless && xvfb-run pnpm playwright test"
fi

pnpm concurrently -krs first "pnpm dev:setup && pnpm dev:up" "wait-on http://localhost:3000 && ${PLAYWRIGHT_CMD} && sleep 3"
