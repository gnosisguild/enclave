#!/bin/sh

echo "loading..."
RUSTFLAGS="-A warnings" cargo run --quiet --bin cmd
