#!/bin/bash
export RUST_LOG=info
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Environment variables
ENVIRONMENT="sepolia"
RPC_URL="ws://localhost:8545"
ENCLAVE_CONTRACT="0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
REGISTRY_CONTRACT="0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
FILTER_REGISTRY_CONTRACT="0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"

# Ciphernode addresses
declare -A ADDRESSES=(
    ["cn1"]="0x2546BcD3c84621e976D8185a91A922aE77ECEc30"
    ["cn2"]="0xbDA5747bFD65F08deb54cb465eB87D40e51B197E"
    ["cn3"]="0xdD2FD4581271e230360230F9337D5c0430Bf44C0"
    ["cn4"]="0x8626f6940E2eb28930eFb4CeF49B2d1F2C9C1199"
)

# Function to create config for a ciphernode
create_config() {
    local name=$1
    local address=${ADDRESSES[$name]}
    local config_file="$SCRIPT_DIR/enclave_data/$name/config.yaml"

    cat << EOF > "$config_file"
config_dir: .
data_dir: .
address: "$address"
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
    echo "$config_file"
}

# Function to run ciphernode
run_ciphernode() {
    local name=$1
    local config_file=$2
    local log_file=$3

    # Set password
    yarn enclave password create \
        --config "$config_file" \
        --password "We are the music makers and we are the dreamers of the dreams."

    # Launch ciphernode
    if [ -n "$log_file" ]; then
        yarn enclave start --config "$config_file" > "$log_file" 2>&1 &
        echo "Started ciphernode $name (PID: $!) - Logging to $log_file"
    else
        yarn enclave start --config "$config_file" &
        echo "Started ciphernode $name (PID: $!)"
    fi
}

# Trap SIGINT (Ctrl + C) to stop all background jobs
trap 'echo "Stopping background processes..."; kill $(jobs -p); exit' SIGINT

# Check if logging is requested
if [ "$1" = "--log" ]; then
    log_to_file=true
else
    log_to_file=false
fi

# Launch all ciphernodes
for name in "${!ADDRESSES[@]}"; do
    DATA_DIR="$SCRIPT_DIR/enclave_data/$name"
    mkdir -p "$DATA_DIR"
    config_file=$(create_config "$name")
    echo "Created config file for $name: $config_file"
    if $log_to_file; then
        run_ciphernode "$name" "$config_file" "$DATA_DIR/ciphernode-$name.log"
    else
        run_ciphernode "$name" "$config_file" 
    fi
done

# If logging to files, tail the logs
if $log_to_file; then
    tail -f enclave_data/*/ciphernode-*.log
else
    # Wait for all background processes
    wait
fi

# Cleanup configs
rm /tmp/*.yaml 2>/dev/null || true
