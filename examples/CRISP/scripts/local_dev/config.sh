#!/bin/bash

# Base directory for node data, logs, configs etc.
export SCRIPT_DIR="/tmp/enclave-nodes"

# Blockchain environment details
export ENVIRONMENT="hardhat"
export RPC_URL="ws://localhost:8545"
export ENCLAVE_CONTRACT="0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512"
export REGISTRY_CONTRACT="0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
export FILTER_REGISTRY_CONTRACT="0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"

# Wallet and Node Security
export PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
export PASSWORD="We are the music makers and we are the dreamers of the dreams."

# Aggregator Configuration
export AGGREGATOR_ADDRESS="0x8626a6940E2eb28930eFb4CeF49B2d1F2C9C1199"
export AGGREGATOR_QUIC_PORT=9204

# CipherNode Configuration
export CIPHERNODE_NAMES=("cn1" "cn2" "cn3") # Used by run_ciphernodes.sh
export CIPHERNODE_ADDRESS_1="0xbDA5747bFD65F08deb54cb465eB87D40e51B197E"
export CIPHERNODE_ADDRESS_2="0xdD2FD4581271e230360230F9337D5c0430Bf44C0"
export CIPHERNODE_ADDRESS_3="0x2546BcD3c84621e976D8185a91A922aE77ECEc30"
export CIPHERNODE_QUIC_PORT_1=9201
export CIPHERNODE_QUIC_PORT_2=9202
export CIPHERNODE_QUIC_PORT_3=9203
export CIPHERNODE_NETWORK="localhost" # Network name for pnpm ciphernode:add

# Define all QUIC ports for peer discovery
export ALL_QUIC_PORTS=($CIPHERNODE_QUIC_PORT_1 $CIPHERNODE_QUIC_PORT_2 $CIPHERNODE_QUIC_PORT_3 $AGGREGATOR_QUIC_PORT) 