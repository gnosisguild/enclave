#!/bin/bash
source ./scripts/local_dev/config.sh
export RUST_LOG=info

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
