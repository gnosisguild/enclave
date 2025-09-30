# !/bin/bash

# Install the enclave binary
cargo install --path ./crates/cli --bin enclave -f

  anvil &
  cd examples/CRISP && enclave wallet set --name ag --private-key "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" && enclave nodes up -v
