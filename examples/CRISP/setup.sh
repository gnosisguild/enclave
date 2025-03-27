#!/usr/bin/env bash

(cd client && yarn)
(cd risc0 && RISC0_SKIP_BUILD=1 cargo check)
(cd server && RISC0_SKIP_BUILD=1 cargo check)
(cd web-rust && cargo check)
