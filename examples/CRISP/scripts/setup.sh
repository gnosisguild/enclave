#!/usr/bin/env bash

set -e

docker compose build 
./scripts/tasks/dockerize.sh ./scripts/tasks/setup.sh

