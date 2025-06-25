#!/usr/bin/env bash

pnpm wasm-pack build --target web --out-dir pkg-web
pnpm wasm-pack build --target nodejs --out-dir pkg-nodejs
