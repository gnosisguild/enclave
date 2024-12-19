#!/usr/bin/env bash

# Enable BuildKit
export DOCKER_BUILDKIT=1

mkdir -p /tmp/docker-cache

time docker buildx build \
  --cache-from=type=local,src=/tmp/docker-cache \
  --cache-to=type=local,dest=/tmp/docker-cache \
  --load \
  -t ghcr.io/gnosisguild/ciphernode -f ./packages/ciphernode/Dockerfile .
