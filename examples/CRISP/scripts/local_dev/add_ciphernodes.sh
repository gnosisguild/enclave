#!/bin/bash
source /app/examples/CRISP/scripts/local_dev/config.sh

# Add ciphernodes using variables from config.sh
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_1 --network $CIPHERNODE_NETWORK
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_2 --network $CIPHERNODE_NETWORK
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_3 --network $CIPHERNODE_NETWORK
