#!/bin/sh

echo "loading..."
RUSTFLAGS="-A warnings" cargo run --quiet --bin node -- --address 0xCc6c693FDB68f0DB58172639CDEa33FF488cf0a5
