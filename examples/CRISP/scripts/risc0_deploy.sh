#!/usr/bin/env bash

(cd ./risc0 && forge script --rpc-url http://localhost:8545 --broadcast script/Deploy.s.sol)
