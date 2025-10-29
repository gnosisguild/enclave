#!/usr/bin/env bash

rm -rf * && git reset --hard HEAD && git submodule update --init --recursive && pnpm install && cargo build && cd examples/CRISP && pnpm test:e2e --ui
