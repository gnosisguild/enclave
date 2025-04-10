#!/usr/bin/env bash

# This script will launch all static components so that someone can run the CRISP protocol locally
set -euxo pipefail

docker compose up -d # ensure our container is running in order to have dev persistence and caching 
docker compose exec enclave-dev ./scripts/dev.sh
