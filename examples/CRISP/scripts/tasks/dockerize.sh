#!/usr/bin/env bash

set -e

cleanup() {
  if [ ! -f /.dockerenv ]; then
    echo "Running docker compose down..."
    docker compose down
    sleep 1
  fi
}

trap cleanup INT TERM

function run_in_docker() {
  # Check if we're already inside a Docker container
  if [ -f /.dockerenv ]; then
    if [ $# -eq 0 ]; then
      # dont do anything if we are already in docker and only request a bash prompt to avoid inception
      exit 0
    fi
    # Already in container, just run the command directly
    echo "Detected running inside container, executing command directly"
    "$@"
  else
    # Not in container, start Docker and run inside
    echo "Running outside container, starting Docker and executing command"
    docker compose up -d # ensure our container is running
     
    if [ $# -eq 0 ]; then
      docker compose exec enclave-dev bash
    else
      docker compose exec enclave-dev "$@"
    fi
  fi
}

run_in_docker "$@"
