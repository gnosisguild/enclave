#!/bin/bash
source ./config.sh
export RUST_LOG=info

# Map names to addresses and ports defined in config.sh
declare -A CIPHERNODE_INFO
CIPHERNODE_INFO["cn1",address]="$CIPHERNODE_ADDRESS_1"
CIPHERNODE_INFO["cn1",port]="$CIPHERNODE_QUIC_PORT_1"
CIPHERNODE_INFO["cn2",address]="$CIPHERNODE_ADDRESS_2"
CIPHERNODE_INFO["cn2",port]="$CIPHERNODE_QUIC_PORT_2"
CIPHERNODE_INFO["cn3",address]="$CIPHERNODE_ADDRESS_3"
CIPHERNODE_INFO["cn3",port]="$CIPHERNODE_QUIC_PORT_3"


# Function to get the address for a ciphernode
get_address() {
    local name="$1"
    if [[ -v CIPHERNODE_INFO[$name,address] ]]; then
        echo "${CIPHERNODE_INFO[$name,address]}"
    else
        echo "Unknown node: $name" >&2
        exit 1
    fi
}

# Function to get the QUIC port for a ciphernode
get_port() {
    local name="$1"
    if [[ -v CIPHERNODE_INFO[$name,port] ]]; then
        echo "${CIPHERNODE_INFO[$name,port]}"
    else
        echo "Unknown node: $name" >&2
        exit 1
    fi
}

# Function to create config for a ciphernode
create_config() {
    local name=$1
    local quic_port
    quic_port=$(get_port "$name") # Get port from map
    local address
    address=$(get_address "$name") # Get address from map
    local config_file="$SCRIPT_DIR/enclave_data/$name/config.yaml"
    
    # Start writing the config file with the standard parts
    cat << EOF > "$config_file"
config_dir: .
data_dir: .
address: "$address"
quic_port: $quic_port
enable_mdns: true
peers:
EOF
    
    # Add each peer from ALL_QUIC_PORTS (defined in config.sh), skipping self
    for port in "${ALL_QUIC_PORTS[@]}"; do
        if [ "$port" -ne "$quic_port" ]; then
            echo "  - \"/ip4/127.0.0.1/udp/$port/quic-v1\"" >> "$config_file"
        fi
    done
    
    # Add the chains section (variables sourced from config.sh)
    cat << EOF >> "$config_file"
chains:
  - name: "$ENVIRONMENT"
    rpc_url: "$RPC_URL"
    contracts:
      enclave: "$ENCLAVE_CONTRACT"
      ciphernode_registry: "$REGISTRY_CONTRACT"
      filter_registry: "$FILTER_REGISTRY_CONTRACT"
EOF
    echo "$config_file"
}

# Function to run ciphernode
run_ciphernode() {
    local name=$1
    local config_file=$2
    local log_file=$3

    # Set password (sourced from config.sh)
    enclave password create \
        --config "$config_file" \
        --password "$PASSWORD"

    # Generate a new key
    enclave net generate-key --config "$config_file"

    # Launch ciphernode
    if [ -n "$log_file" ]; then
        enclave start --config "$config_file" > "$log_file" 2>&1 &
        echo "Started ciphernode $name (PID: $!) - Logging to $log_file"
    else
        enclave start --config "$config_file" &
        echo "Started ciphernode $name (PID: $!)"
    fi
}

# Trap SIGINT (Ctrl + C) to stop all background jobs
trap 'echo "Stopping background processes..."; kill $(jobs -p 2>/dev/null); exit' SIGINT

# Check if logging is requested
if [ "$1" = "--log" ]; then
    log_to_file=true
else
    log_to_file=false
fi

# Launch all ciphernodes using names from config.sh
for name in "${CIPHERNODE_NAMES[@]}"; do
    DATA_DIR="$SCRIPT_DIR/enclave_data/$name"
    mkdir -p "$DATA_DIR"
    config_file=$(create_config "$name") # Removed quic_port increment
    echo "Created config file for $name: $config_file"
    if $log_to_file; then
        run_ciphernode "$name" "$config_file" "$DATA_DIR/ciphernode-$name.log"
    else
        run_ciphernode "$name" "$config_file" 
    fi
done

# If logging to files, tail the logs
if $log_to_file; then
    tail -f $SCRIPT_DIR/enclave_data/*/ciphernode-*.log
else
    # Wait for all background processes
    wait
fi

# Cleanup configs
rm /tmp/*.yaml 2>/dev/null || true
