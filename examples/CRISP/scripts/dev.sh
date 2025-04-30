#!/usr/bin/env bash

# This script will launch all static components so that someone can run the CRISP protocol locally
set -euxo pipefail

source ./scripts/shared.sh

run_in_docker ./scripts/tasks/dev.sh
