#!/usr/bin/env bash

RUSTFLAGS="-A warnings" cargo run --bin aggregator -- "$@"
