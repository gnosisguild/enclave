#!/bin/bash

# Convenience wrapper - forwards to scripts/run_benchmarks.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "${SCRIPT_DIR}/scripts/run_benchmarks.sh" "$@"
