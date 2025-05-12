#!/usr/bin/env bash

set -e

pnpm synpress
pnpm concurrently -krs first "pnpm dev:setup && pnpm dev:up" "wait-on http://localhost:3000 && playwright test $@&& sleep 3"
