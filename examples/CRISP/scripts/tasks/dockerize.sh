#!/usr/bin/env bash

set -x

cleanup() {
  if [ ! -f /.dockerenv ]; then
    echo "Running docker compose down..."
    docker compose down
    sleep 1
  fi
}

trap cleanup INT TERM

function run_in_docker() {
    # Check if any arguments were provided
    if [ $# -eq 0 ]; then
        echo "Error: No arguments provided"
        echo "Usage: run_in_docker <command_and_args>"
        return 1
    fi
    
    # Check if we're already inside a Docker container
    if [ -f /.dockerenv ]; then
        # Already in container, just run the command directly
        echo "Detected running inside container, executing command directly"
        "$@"
    else
        # Not in container, start Docker and run inside
        echo "Running outside container, starting Docker and executing command"
        docker compose up -d # ensure our container is running
        
        # Pass all arguments to docker compose exec
        docker compose exec enclave-dev "$@"
    fi
}

run_in_docker "$@"
