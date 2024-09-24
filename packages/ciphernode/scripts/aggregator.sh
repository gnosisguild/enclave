#!/bin/sh

RUSTFLAGS="-A warnings" cargo run --bin aggregator -- $@
