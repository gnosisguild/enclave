#!/usr/bin/env bash

TAG=${1:-latest}

docker build -t ghcr.io/gnosisguild/ciphernode:$TAG -f ./packages/ciphernode/Dockerfile .
