#!/usr/bin/env sh

RUSTFLAGS="-A warnings" cargo run --bin test_encryptor -- $@
