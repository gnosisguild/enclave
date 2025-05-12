#!/usr/bin/env bash

set -e

setup_file="/tmp/.setup_script_ran"

# Check for --force flag
force=false
for arg in "$@"; do
  if [ "$arg" = "--force" ]; then
    force=true
    break
  fi
done

# If force flag is present, remove the setup file
if [ "$force" = true ] && [ -f "$setup_file" ]; then
  rm -f "$setup_file"
  echo "Force flag detected. Removed existing setup file."
fi

if [ -f $setup_file ]; then 
  echo "already run"
  exit 0
fi

if command -v docker &> /dev/null; then
    docker compose build 
fi

./scripts/tasks/dockerize.sh ./scripts/tasks/setup.sh

echo 1 > $setup_file

