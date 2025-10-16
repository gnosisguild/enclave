# !/bin/bash

# Install the enclave binary
# cargo install --locked --path ./crates/cli --bin enclave -f

# Deploy Contacts
(cd packages/enclave-contracts && rm -rf deployments/localhost && pnpm deploy:mocks --network localhost)

# Deploy CRISP Contracts
(cd examples/CRISP && ETH_WALLET_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 FOUNDRY_PROFILE=local forge script --rpc-url http://localhost:8545 --broadcast deploy/Deploy.s.sol)

# Add Ciphernodes to Enclave
sleep 2 # wait for enclave to start

# Get the addresses of the ciphernodes
CN1=0xbDA5747bFD65F08deb54cb465eB87D40e51B197E
CN2=0xdD2FD4581271e230360230F9337D5c0430Bf44C0
CN3=0x2546BcD3c84621e976D8185a91A922aE77ECEc30

# Add the ciphernodes to the enclave
pnpm ciphernode:add --ciphernode-address "$CN1" --network "localhost"
pnpm ciphernode:add --ciphernode-address "$CN2" --network "localhost"
pnpm ciphernode:add --ciphernode-address "$CN3" --network "localhost"


# Delete local DB
(rm -rf ./examples/CRISP/server/database)
