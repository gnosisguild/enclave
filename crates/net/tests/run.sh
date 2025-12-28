#!/usr/bin/env bash
set -e
# Export env vars once for all docker compose commands
export DOCKER_BUILDKIT=1
export COMPOSE_DOCKER_CLI_BUILD=1

# Get the current commit SHA
export IMAGE_TAG=$(git rev-parse --short HEAD)

echo ""
echo "Building docker image (p2p_test:${IMAGE_TAG})"
echo ""
docker build --network host -f ./Dockerfile -t "p2p_test:${IMAGE_TAG}" ../../..
echo ""
echo "NETWORK TESTS"
echo ""
docker compose up --abort-on-container-exit
