#!/usr/bin/env bash

if [ ! -f "./.deploy/.env" ]; then
    echo "Environment file ./.deploy/.env not found!"
    exit 1
fi

source .deploy/.env

docker stack deploy -c .deploy/docker-compose.yml enclave-stack --detach=false
