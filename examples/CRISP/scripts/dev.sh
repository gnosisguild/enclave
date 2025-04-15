#!/usr/bin/env bash

# This script will launch all static components so that someone can run the CRISP protocol locally
set -euxo pipefail

docker compose -f ../docker-compose.yaml up -d # ensure our container is running in order to have dev persistence and caching 
docker compose -f ../docker-compose.yaml exec enclave-dev ./tasks/dev.sh
