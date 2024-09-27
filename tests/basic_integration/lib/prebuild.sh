#!/usr/bin/env sh

cd packages/ciphernode && RUSTFLAGS="-A warnings" cargo build --bin fake_encrypt --bin node --bin aggregator;
