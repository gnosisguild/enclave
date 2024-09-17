#!/bin/sh

echo "loading..."
RUSTFLAGS="-A warnings" cargo run --quiet --bin node -- --address 0x25c693E1188b9E4455E07DC4f6a49142eFbF2C61
