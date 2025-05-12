#!/usr/bin/env bash

set -e

if command -v docker &> /dev/null; then
    time docker compose build 
    echo "#### docker compose build finished ####"
fi

time ./scripts/tasks/dockerize.sh ./scripts/tasks/setup.sh
echo "#### ./scripts/tasks/setup.sh finished ####"


