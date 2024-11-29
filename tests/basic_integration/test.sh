#!/usr/bin/env bash

set -eu  # Exit immediately if a command exits with a non-zero status

THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

if [ $# -eq 0 ]; then 
  "$THIS_DIR/persist.sh"
  "$THIS_DIR/base.sh"
  "$THIS_DIR/net.sh"
else
  "$THIS_DIR/$1.sh"
fi

