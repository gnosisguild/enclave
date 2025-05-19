#!/usr/bin/env bash

set -e

pushd ./scripts/build_fixtures.sh && popd

cargo test -- $@
