#!/usr/bin/env bash

set -e

(cd ./apps/server && ./target/debug/cli "$@")
