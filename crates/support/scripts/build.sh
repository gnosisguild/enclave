#!/usr/bin/env bash
PKG=ghcr.io/gnosisguild/e3-support
GIT_SHA=$(git rev-parse --short HEAD)

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
docker build -t $PKG:$GIT_SHA "${BUILD_ARGS[@]}" .

# Push if --push was specified
if [ "$PUSH" = true ]; then
  docker push $PKG:$GIT_SHA
  echo "Image pushed to: $PKG:$GIT_SHA"
fi
