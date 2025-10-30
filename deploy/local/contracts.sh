# !/bin/bash

# Install the enclave binary
# cargo install --locked --path ./crates/cli --bin enclave -f

# Deploy CRISP Contracts
(cd examples/CRISP/packages/crisp-contracts && USE_MOCK_VERIFIER=true pnpm deploy:contracts:full --network localhost)

# Add Ciphernodes to Enclave
sleep 2 # wait for enclave to start

# Get the addresses of the ciphernodes
CN1=0x70997970C51812dc3A010C7d01b50e0d17dc79C8
CN2=0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC
CN3=0x90F79bf6EB2c4f870365E785982E1f101E93b906

# Add the ciphernodes to the enclave
(cd examples/CRISP/packages/crisp-contracts && pnpm ciphernode:add --ciphernode-address "$CN1" --network "localhost")
(cd examples/CRISP/packages/crisp-contracts && pnpm ciphernode:add --ciphernode-address "$CN2" --network "localhost")
(cd examples/CRISP/packages/crisp-contracts && pnpm ciphernode:add --ciphernode-address "$CN3" --network "localhost")


# Delete local DB
(rm -rf ./examples/CRISP/server/database)
