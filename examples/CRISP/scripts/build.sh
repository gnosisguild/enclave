#!/usr/bin/env bash

set -e

(cd evm && yarn compile)
(cd web-rust && cargo build)
(cd risc0 && cargo build)
(cd server && cargo build)
(cd client && yarn build)
