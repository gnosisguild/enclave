#!/bin/sh 

# Environment variables
export RPC_URL="ws://localhost:8545"
export ENCLAVE_CONTRACT="0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
export REGISTRY_CONTRACT="0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
export CIPHERNODE_ADDRESS_1="0x2546BcD3c84621e976D8185a91A922aE77ECEc30"
export CIPHERNODE_ADDRESS_2="0xbDA5747bFD65F08deb54cb465eB87D40e51B197E"
export CIPHERNODE_ADDRESS_3="0xdD2FD4581271e230360230F9337D5c0430Bf44C0"
export CIPHERNODE_ADDRESS_4="0x8626f6940E2eb28930eFb4CeF49B2d1F2C9C1199"

# Function to clean up background processes
cleanup() {
    echo "Cleaning up processes..."
    kill $(jobs -p)
    exit
}

# Set up trap to catch Ctrl+C
trap cleanup SIGINT

# pushd packages/ciphernode && \
#   cargo build --bin node && \
#   cargo build --bin aggregator && \
#   popd

# Start the EVM node
yarn evm:node &
sleep 2



# Launch ciphernodes in parallel
yarn ciphernode:launch --address $CIPHERNODE_ADDRESS_1 --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT &
yarn ciphernode:launch --address $CIPHERNODE_ADDRESS_2 --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT &
yarn ciphernode:launch --address $CIPHERNODE_ADDRESS_3 --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT &
yarn ciphernode:launch --address $CIPHERNODE_ADDRESS_4 --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT &
yarn ciphernode:aggregator --rpc "$RPC_URL" --enclave-contract $ENCLAVE_CONTRACT --registry-contract $REGISTRY_CONTRACT --pubkey-write-path "../../pubkey.b64" &

sleep 2

# Run ciphernode:add commands sequentially
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_1 --network localhost
sleep 2
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_2 --network localhost
sleep 2
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_3 --network localhost
sleep 2
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_4 --network localhost

# Wait for Ctrl+C
echo "All processes are running. Press Ctrl+C to stop and clean up."
wait
