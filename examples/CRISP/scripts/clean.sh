#!/usr/bin/env bash

# This script will clean caches and remove images
set -e

docker compose -f ../docker-compose.yaml down
