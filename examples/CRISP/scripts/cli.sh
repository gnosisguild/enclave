#!/usr/bin/env bash

set -e

./scripts/tasks/dockerize.sh ./scripts/tasks/cli.sh "$@"

