#!/bin/bash

set -e

export PRIVATE_KEY=${PRIVATE_KEY:-0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80}
export RPC_URL=${RPC_URL:-"http://localhost:8545"}

echo "Cleaning previous deployments..."
rm -rf deployments/localhost
rm -rf deployments/core
rm -rf broadcast
echo ""

echo "1. Deploying EigenLayer Core Contracts"
echo "======================================"

forge script deploy/eigenlayer/DeployEigenLayerCore.s.sol \
    --rpc-url "$RPC_URL" \
    --broadcast 

if [ $? -ne 0 ]; then
    echo "EigenLayer core deployment failed"
    exit 1
fi

echo "EigenLayer core contracts deployed successfully"
echo ""

echo "2. Deploying Complete Enclave System"  
echo "===================================="

npx hardhat deploy --network localhost --tags enclave,tokenomics,post,mocks

if [ $? -ne 0 ]; then
    echo "Enclave system deployment failed"
    exit 1
fi