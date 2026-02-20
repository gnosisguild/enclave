#!/usr/bin/env bash

set -e

echo "Building nargo dependencies..."

cd ../../circuits/bin/dkg && nargo build
