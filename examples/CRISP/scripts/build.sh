#!/usr/bin/env bash

# This script should build all binaries so that CRISP can be deployed 
set -e

source ./scripts/shared.sh

run_in_docker ./scripts/tasks/build.sh





