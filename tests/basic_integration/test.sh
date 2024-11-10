#!/usr/bin/env bash

set -eu  # Exit immediately if a command exits with a non-zero status

THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Source the file from the same directory
case $1 in 
  persist)
    "$THIS_DIR/persist.sh"
    ;;
  base)
    "$THIS_DIR/base.sh"
    ;;
  *)
    "$THIS_DIR/persist.sh"
    "$THIS_DIR/base.sh"
    ;;
esac

