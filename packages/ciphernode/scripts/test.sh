#!/usr/bin/env bash

set -e

pushd ./evm && ./scripts/build_fixtures.sh && popd
pushd ./evm_helpers && ./scripts/build_fixtures.sh && popd
pushd ./indexer && ./scripts/build_fixtures.sh && popd

cargo test -- $@
