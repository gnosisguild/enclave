#!/usr/bin/env bash

if [ ! -f ".env" ]; then
    echo "Environment file .env not found!"
    exit 1
fi

# Source the environment file
set -a  # automatically export all variables
source .deploy/.env
set +a  # turn off auto-export

docker stack deploy -c .deploy/docker-compose.yml enclave-stack --detach=false
