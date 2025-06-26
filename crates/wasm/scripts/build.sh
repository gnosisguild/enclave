#!/usr/bin/env bash

pnpm wasm-pack build --target web --out-dir dist/web
pnpm wasm-pack build --target nodejs --out-dir dist/nodejs
