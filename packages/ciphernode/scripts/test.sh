#!/usr/bin/env bash

set -e

pushd ./evm && ./scripts/build_fixtures.sh && popd

cargo test -- $@
