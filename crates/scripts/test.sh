#!/usr/bin/env bash

set -e

./scripts/build_fixtures.sh

cargo test -- $@
