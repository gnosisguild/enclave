#!/usr/bin/env bash

set -x

function run_in_docker() {
    # Check if any arguments were provided
    if [ $# -eq 0 ]; then
        echo "Error: No arguments provided"
        echo "Usage: run_in_docker <command_and_args>"
        return 1
    fi
    
    # Check if we're already inside a Docker container
    if [ -f /.dockerenv ] || grep -q 'docker\|lxc' /proc/1/cgroup 2>/dev/null; then
        # Already in container, just run the command directly
        echo "Detected running inside container, executing command directly"
        "$@"
    else
        # Not in container, start Docker and run inside
        echo "Running outside container, starting Docker and executing command"
        docker compose up -d # ensure our container is running
        
        # Pass all arguments to docker compose exec
        docker compose exec enclave-dev "cd /app/examples/CRISP && $@"
    fi
}

run_in_docker "$@"
