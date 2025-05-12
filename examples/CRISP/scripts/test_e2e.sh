#!/usr/bin/env bash

set -e

if [ "$1" == "--ui" ]; then
  pnpm concurrently -krs first "pnpm dev:setup && pnpm dev:up" "wait-on http://localhost:3000 && pnpm synpress && pnpm playwright test --ui && sleep 3"
else
  pnpm concurrently -krs first "pnpm dev:setup && pnpm dev:up" "wait-on http://localhost:3000 && pnpm synpress --headless && pnpm playwright test
 && sleep 3"
fi
