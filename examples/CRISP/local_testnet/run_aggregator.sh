#!/bin/bash
export RUST_LOG=info

# Environment variables
ENVIRONMENT="hardhat"
RPC_URL="ws://localhost:8545"
ENCLAVE_CONTRACT="0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
REGISTRY_CONTRACT="0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
FILTER_REGISTRY_CONTRACT="0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
NODE_ADDRESS="0x8626a6940E2eb28930eFb4CeF49B2d1F2C9C1199"
PASSWORD="We are the music makers and we are the dreamers of the dreams."
QUIC_PORT=9204

# Setup directories
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DATA_DIR="$SCRIPT_DIR/enclave_data/aggregator"
LOG_FILE="$DATA_DIR/aggregator.log"

mkdir -p "$DATA_DIR"

# Create a temporary config file
CONFIG_FILE="$DATA_DIR/config.yaml"
cat << EOF > "$CONFIG_FILE"
config_dir: . 
data_dir: .
address: "$NODE_ADDRESS"
quic_port: $quic_port
enable_mdns: true
chains:
  - name: "$ENVIRONMENT"
    rpc_url: "$RPC_URL"
    contracts:
      enclave: "$ENCLAVE_CONTRACT"
      ciphernode_registry: "$REGISTRY_CONTRACT"
      filter_registry: "$FILTER_REGISTRY_CONTRACT"
EOF

# Trap SIGINT (Ctrl + C) to stop all background jobs
trap 'echo "Stopping background processes..."; kill -- -$$' SIGINT

# Set password
yarn enclave password create --config "$CONFIG_FILE" --password "$PASSWORD"

# Set network key
yarn enclave net generate-key --config "$CONFIG_FILE"

# Set private key for the wallet
yarn enclave wallet set --config "$CONFIG_FILE" --private-key "$PRIVATE_KEY"

# Run the aggregator in the background
yarn enclave aggregator start --config "$CONFIG_FILE" &

# Wait for all background processes to finish
wait
