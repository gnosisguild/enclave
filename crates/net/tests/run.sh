#!/usr/bin/env bash

set -e

# Export env vars once for all docker compose commands
export DOCKER_BUILDKIT=1
export COMPOSE_DOCKER_CLI_BUILD=1

echo ""
echo "Building docker image"
echo ""
docker build --network host -f ./Dockerfile -t p2p_test:latest ../../..

echo ""
echo "NETWORK TESTS"
echo ""
docker compose up --abort-on-container-exit
