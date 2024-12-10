#!/usr/bin/env bash

IMAGE_NAME=ghcr.io/gnosisguild/ciphernode
VERSION=$(git rev-parse HEAD)
DOCKERFILE_PATH=packages/ciphernode/Dockerfile

docker build -t $IMAGE_NAME:${VERSION} -f $DOCKERFILE_PATH .
