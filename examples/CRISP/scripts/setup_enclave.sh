#!/usr/bin/env bash

rm -rf /tmp/enclave
git clone --depth=1 https://github.com/gnosisguild/enclave.git /tmp/enclave
(cd /tmp/enclave/packages/evm && yarn && yarn compile)
(cd /tmp/enclave/packages/ciphernode && cargo build && cargo install --path ./enclave)
