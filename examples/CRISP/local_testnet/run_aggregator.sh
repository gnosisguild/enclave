#!/bin/bash
export RUST_LOG=info

# Environment variables
ENVIRONMENT="hardhat"
PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
RPC_URL="ws://localhost:8545"
ENCLAVE_CONTRACT="0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
REGISTRY_CONTRACT="0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
FILTER_REGISTRY_CONTRACT="0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"

# Setup directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p enclave_data/aggregator
DATA_DIR="$SCRIPT_DIR/enclave_data/aggregator"
LOG_FILE="$DATA_DIR/aggregator.log"

# Create a temporary config file
CONFIG_FILE="$DATA_DIR/config.yaml"
cat << EOF > "$CONFIG_FILE"
config_dir: . 
data_dir: .
address: "0x514910771AF9Ca656af840dff83E8264EcF986CA"
chains:
  - name: "$ENVIRONMENT"
    rpc_url: "$RPC_URL"
    contracts:
      enclave: 
        address: "$ENCLAVE_CONTRACT"
        deploy_block: 0
      ciphernode_registry: "$REGISTRY_CONTRACT"
      filter_registry: "$FILTER_REGISTRY_CONTRACT"
EOF

# Trap SIGINT (Ctrl + C) and stop all background jobs
trap 'echo "Stopping background processes..."; kill $(jobs -p); exit' SIGINT

# Set password and private key
yarn enclave password create --config "$CONFIG_FILE" --password "We are the music makers and we are the dreamers of the dreams."
yarn enclave wallet set --config "$CONFIG_FILE" --private-key "$PRIVATE_KEY"

# Run the aggregator in the background
yarn enclave aggregator start \
  --config "$CONFIG_FILE" \
  --pubkey-write-path "$DATA_DIR/pubkey.bin" \
  --plaintext-write-path "$DATA_DIR/plaintext.txt" 
# Wait for all background processes to finish
wait