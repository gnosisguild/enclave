#!/usr/bin/env bash

cd packages/ciphernode && RUSTFLAGS="-A warnings" cargo run --bin pack_e3_params -- "$@" 
