#!/usr/bin/env bash

cd packages/ciphernode && cargo run --bin pack_e3_params -- "$@" 
