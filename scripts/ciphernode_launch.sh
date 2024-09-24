#!/bin/sh 

pushd packages/ciphernode && yarn ciphernode:add --ciphernode-address $1
pushd packages/evm && ./scripts/launch.sh --address $1
