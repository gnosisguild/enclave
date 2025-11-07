#!/bin/bash
set -e

cleanup() {
    kill -- -$$  # Kill entire process group
    exit 1
}

trap cleanup SIGTERM SIGINT SIGKILL

wait-on tcp:3000
exec pnpm test:sdk
