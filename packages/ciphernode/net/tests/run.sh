#!/usr/bin/env bash

set -e

echo ""
echo "Building docker image"
echo ""
docker compose build

echo ""
echo "TEST 1: Using MDNS with separate IP addresses"
echo ""
docker compose up --build --abort-on-container-exit

echo ""
echo "TEST 2: Blocking MDNS traffic for each service"
echo ""
echo "Note this should display some libp2p_mdns::behaviour::iface errors in output"
echo ""
BLOCK_MDNS=true docker compose up --build --abort-on-container-exit
