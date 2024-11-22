#!/usr/bin/env bash
pkill p2p_messaging
trap 'killall p2p_test 2>/dev/null' EXIT

cargo run --bin p2p_test alice &
cargo run --bin p2p_test bob & 
cargo run --bin p2p_test charlie &

# Wait for all background processes to complete
wait
