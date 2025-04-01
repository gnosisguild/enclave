#!/usr/bin/env bash

set -e

# This is all stuff that has to happen after the source code is mounted 
# TOOD: perhaps we can try and move more of this to the dockerfile build process
(cd client && yarn)
(git clone --depth=1 https://github.com/gnosisguild/enclave.git && cd ./enclave/packages/evm && yarn)
(cd risc0 && yarn && cargo check)
(cd server && [[ ! -f .env ]] && cp .env.example .env; cargo check)
(cd web-rust && cargo check)
