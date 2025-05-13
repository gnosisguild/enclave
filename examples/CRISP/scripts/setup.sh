#!/usr/bin/env bash

set -e

if command -v docker &> /dev/null; then
    docker compose build 
fi

./scripts/tasks/dockerize.sh ./scripts/tasks/setup.sh
