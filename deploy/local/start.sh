#!/bin/bash

set -e

echo "🚀 Starting CRISP Development Environment..."

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check dependencies
echo "📋 Checking dependencies..."

if ! command_exists "cargo"; then
    echo "❌ Rust/Cargo is required but not installed"
    exit 1
fi

if ! command_exists "pnpm"; then
    echo "❌ pnpm is required but not installed"
    exit 1
fi

if ! command_exists "concurrently"; then
    echo "❌ concurrently is required but not installed"
    echo "Install with: npm install -g concurrently"
    exit 1
fi

if ! command_exists "anvil"; then
    echo "❌ Foundry/Anvil is required but not installed"
    exit 1
fi

echo "✅ All dependencies found"

# Install the enclave binary
echo "🔧 Installing Enclave CLI..."
cargo install --locked --path ./crates/cli --bin enclave -f

# Function to wait for a service to be ready
wait_for_service() {
    local url=$1
    local service_name=$2
    local max_attempts=30
    local attempt=1
    
    echo "⏳ Waiting for $service_name to be ready..."
    
    while [ $attempt -le $max_attempts ]; do
        if curl -s "$url" >/dev/null 2>&1; then
            echo "✅ $service_name is ready!"
            return 0
        fi
        echo "   Attempt $attempt/$max_attempts - $service_name not ready yet..."
        sleep 2
        attempt=$((attempt + 1))
    done
    
    echo "❌ $service_name failed to start after $max_attempts attempts"
    return 1
}

# Function to deploy contracts
deploy_contracts() {
    echo "📄 Deploying contracts..."
    
    # Deploy Enclave contracts
    echo "   Deploying Enclave contracts..."
    (cd packages/enclave-contracts && rm -rf deployments/localhost && pnpm deploy:mocks --network localhost)
    
    # Deploy CRISP contracts
    echo "   Deploying CRISP contracts..."
    (cd examples/CRISP/packages/crisp-contracts && ETH_WALLET_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 FOUNDRY_PROFILE=local forge script --rpc-url http://localhost:8545 --broadcast deploy/Deploy.s.sol)
    
    # Wait a bit for nodes to be ready
    sleep 5
    
    # Add ciphernodes to the registry
    echo "   Adding ciphernodes to registry..."
    CN1=0xbDA5747bFD65F08deb54cb465eB87D40e51B197E
    CN2=0xdD2FD4581271e230360230F9337D5c0430Bf44C0
    CN3=0x2546BcD3c84621e976D8185a91A922aE77ECEc30
    
    pnpm ciphernode:add --ciphernode-address "$CN1" --network "localhost"
    pnpm ciphernode:add --ciphernode-address "$CN2" --network "localhost"
    pnpm ciphernode:add --ciphernode-address "$CN3" --network "localhost"
    
    # Clean up local database
    echo "   Cleaning up local database..."
    rm -rf ./examples/CRISP/server/database
    
    echo "✅ Contracts deployed successfully!"
}

# Start infrastructure (anvil + ciphernodes) in background
echo "🏗️  Starting infrastructure..."
concurrently \
  --names "ANVIL,NODES" \
  --prefix-colors "blue,yellow" \
  "anvil" \
  "cd examples/CRISP && enclave wallet set --name ag --private-key '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80' && enclave nodes up -v" &

INFRA_PID=$!

# Wait for Anvil to be ready
wait_for_service "http://localhost:8545" "Anvil"

# Deploy contracts
deploy_contracts

# Wait a moment for everything to stabilize
echo "⏳ Waiting for infrastructure to stabilize..."
sleep 3

# Install CRISP dependencies
echo "📦 Installing CRISP dependencies..."
(cd examples/CRISP/client && pnpm install)

echo "🎯 Starting CRISP applications..."

# Start all CRISP applications
concurrently \
  --names "CLIENT,SERVER,PROGRAM" \
  --prefix-colors "green,yellow,magenta" \
  "cd examples/CRISP/client && pnpm dev" \
  "cd examples/CRISP/server && cargo run --bin server" \
  "cd examples/CRISP/program && cargo run"

# This will run until interrupted
echo "🚨 CRISP development environment stopped" 