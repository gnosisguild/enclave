#!/usr/bin/env bash
set -euo pipefail

cd circuits/lib
nargo test

echo "Noir circuits tested successfully"