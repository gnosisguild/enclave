#!/usr/bin/env bash
PKG=ghcr.io/gnosisguild/e3-support
GIT_SHA=$(git rev-parse --short HEAD)
docker build -t $PKG:$GIT_SHA -t $PKG:next .
if [ "$1" = "--push" ]; then
  docker push $PKG:$GIT_SHA
  docker push $PKG:next
  echo "Image pushed to: $PKG:$GIT_SHA"
  echo "Image pushed to: $PKG:next"
fi
