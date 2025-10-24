# !/bin/bash

# Install the enclave binary
# cargo install --locked --path ./crates/cli --bin enclave -f

concurrently \
  --names "ANVIL,NODES" \
  --prefix-colors "blue,yellow" \
  "anvil" \
  "cd examples/CRISP && \
  enclave wallet set --name ag --private-key "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" && 
  enclave wallet set --name cn1 --private-key "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" && 
  enclave wallet set --name cn2 --private-key "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a" && 
  enclave wallet set --name cn3 --private-key "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6" &&
  enclave nodes up -v"