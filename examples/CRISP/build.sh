#!/usr/bin/env bash

# This script should build all binaries so that CRISP can be deployed 
set -e

docker compose up -d # ensure our container is running in order to have dev persistence and caching 
docker compose exec enclave-dev ./scripts/build.sh
