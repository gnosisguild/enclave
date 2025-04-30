#!/usr/bin/env bash

# This script is designed to setup and install all dependencies within the system

set -euxo pipefail

# build to ensure the image exists
docker compose build 

source ./scripts/shared.sh

run_in_docker ./scripts/tasks/setup.sh

