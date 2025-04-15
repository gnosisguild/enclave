#!/usr/bin/env bash

# This script is designed to setup and install all dependencies within the system

set -euxo pipefail

docker compose -f ../docker-compose.yaml build 
docker compose -f ../docker-compose.yaml up -d # ensure our container is running in order to have dev persistence and caching 
docker compose -f ../docker-compose.yaml exec enclave-dev ./tasks/setup.sh
