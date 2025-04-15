#!/bin/bash
source ./config.sh
export RUST_LOG=info

# Environment variables - Sourced from config.sh
# ENVIRONMENT="hardhat"
# RPC_URL="ws://localhost:8545"
# ENCLAVE_CONTRACT="0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
# REGISTRY_CONTRACT="0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
# FILTER_REGISTRY_CONTRACT="0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
# PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
# NODE_ADDRESS="0x8626a6940E2eb28930eFb4CeF49B2d1F2C9C1199"
# PASSWORD="We are the music makers and we are the dreamers of the dreams."
# QUIC_PORT=9204

# Setup directories - SCRIPT_DIR sourced from config.sh
# SCRIPT_DIR=/tmp/enclave-nodes
DATA_DIR="$SCRIPT_DIR/enclave_data/aggregator"
LOG_FILE="$DATA_DIR/aggregator.log"

mkdir -p "$DATA_DIR"

# Create a temporary config file
CONFIG_FILE="$DATA_DIR/config.yaml"
cat << EOF > "$CONFIG_FILE"
config_dir: . 
data_dir: .
address: "$AGGREGATOR_ADDRESS" # Use AGGREGATOR_ADDRESS from config.sh
quic_port: $AGGREGATOR_QUIC_PORT # Use AGGREGATOR_QUIC_PORT from config.sh
enable_mdns: true
peers:
EOF

# Add each peer using the ALL_QUIC_PORTS array from config.sh
for port in "${ALL_QUIC_PORTS[@]}"; do
    # Don't add self as peer
    if [ "$port" -ne "$AGGREGATOR_QUIC_PORT" ]; then
        echo "  - \"/ip4/127.0.0.1/udp/$port/quic-v1\"" >> "$CONFIG_FILE"
    fi
done

# Add the chains section (variables sourced from config.sh)
cat << EOF >> "$CONFIG_FILE"
chains:
  - name: "$ENVIRONMENT"
    rpc_url: "$RPC_URL"
EOF

# Trap SIGINT (Ctrl + C) to stop all background jobs
trap 'echo "Stopping background processes..."; kill -- -$$' SIGINT

# Set password
enclave password create --config "$CONFIG_FILE" --password "$PASSWORD"

# Set network key
enclave net generate-key --config "$CONFIG_FILE"

# Set private key for the wallet
enclave wallet set --config "$CONFIG_FILE" --private-key "$PRIVATE_KEY"

# Run the aggregator in the background
enclave aggregator start --config "$CONFIG_FILE" &

# Wait for all background processes to finish
wait
