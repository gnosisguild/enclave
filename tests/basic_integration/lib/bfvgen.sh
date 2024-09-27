#!/usr/bin/env bash

cd packages/ciphernode && RUSTFLAGS="-A warnings" cargo run --bin bfvgen -- "$@" 
