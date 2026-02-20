#!/usr/bin/env bash

set -e


cd "$(git rev-parse --show-toplevel)"

# if this is a clean checkout we need to have some artifacts to test against
if find ./circuits/bin -name '*.json' -print -quit | grep -q .; then
  exit 0
fi

# if we are in CI where circuits have been built ignore
if ! command -v nargo &> /dev/null; then
    exit 0
fi

echo "Building circuits..."

pnpm install && pnpm build:circuits
