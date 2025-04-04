#!/usr/bin/env bash

# This script is designed to setup and install all dependencies within the system

set -euxo pipefail

docker compose build
docker compose up -d # ensure our container is running in order to have dev persistence and caching 
docker compose exec enclave-dev ./scripts/setup.sh
# docker compose exec enclave-dev bash
