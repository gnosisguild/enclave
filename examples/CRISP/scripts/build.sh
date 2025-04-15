#!/usr/bin/env bash

# This script should build all binaries so that CRISP can be deployed 
set -e

docker compose -f ../docker-compose.yaml up -d # ensure our container is running in order to have dev persistence and caching 
docker compose -f ../docker-compose.yaml exec enclave-dev ./tasks/build.sh
