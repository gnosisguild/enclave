#!/usr/bin/env bash

cd packages/ciphernode && cargo run --bin fake_encrypt -- "$@" 
