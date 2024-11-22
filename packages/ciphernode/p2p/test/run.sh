#!/usr/bin/env bash
pkill p2p_messaging
trap 'killall p2p_messaging 2>/dev/null' EXIT

cargo run --bin p2p_messaging alice &
cargo run --bin p2p_messaging bob & 
cargo run --bin p2p_messaging charlie &

# Wait for all background processes to complete
wait

