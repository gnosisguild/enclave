#!/usr/bin/env bash

source .deploy/.env

docker stack deploy -c .deploy/docker-compose.yml enclave-stack --detach=false
