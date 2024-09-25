#!/usr/bin/env sh

RUSTFLAGS="-A warnings" cargo run --bin aggregator -- $@
