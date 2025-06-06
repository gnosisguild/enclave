#!/usr/bin/env bash

./node_modules/.bin/hardhat run "./scripts/ciphernode-add.ts --ciphernode-address $1" --network "$2"
