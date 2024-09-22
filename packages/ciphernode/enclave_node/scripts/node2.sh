#!/bin/sh

echo "loading..."
RUSTFLAGS="-A warnings" cargo run --quiet --bin node -- --address 0x75437e59cAC691C0624e089554834619dc49B944
