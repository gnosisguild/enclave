#!/bin/sh

echo "loading..."
RUSTFLAGS="-A warnings" cargo run --quiet --bin node -- --address 0xe3092f4A2B59234a557aa2dE5D97314D4E969764
