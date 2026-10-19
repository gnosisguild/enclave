#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-3.0-only
#
# Builds the e3-support image, CRISP's program runner (examples/CRISP/runner/Dockerfile), from the
# repository root. `--push` also pushes it. Other arguments go to `docker build`.
PKG="${E3_SUPPORT_IMAGE_REPOSITORY:-ghcr.io/theinterfold/e3-support}"
GIT_SHA=$(git rev-parse --short=9 HEAD)

# Separate --push from other arguments
PUSH=false
BUILD_ARGS=()

for arg in "$@"; do
  if [ "$arg" = "--push" ]; then
    PUSH=true
  else
    BUILD_ARGS+=("$arg")
  fi
done

# Build with any additional arguments
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
docker build -t "$PKG:$GIT_SHA" -f "$ROOT/examples/CRISP/runner/Dockerfile" "${BUILD_ARGS[@]}" "$ROOT"

# Push if --push was specified
if [ "$PUSH" = true ]; then
  docker push "$PKG:$GIT_SHA"
  echo "Image pushed to: $PKG:$GIT_SHA"
fi
