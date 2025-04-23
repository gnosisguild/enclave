#!/usr/bin/env bash

set -euo pipefail

PASSWORD="This is a password"
PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"


# TODO: automate this
enclave password create --name cn1 --password "$PASSWORD"
enclave password create --name cn2 --password "$PASSWORD"
enclave password create --name cn3 --password "$PASSWORD"
enclave password create --name ag --password "$PASSWORD"
enclave net generate-key --name cn1
enclave net generate-key --name cn2
enclave net generate-key --name cn3
enclave net generate-key --name ag
enclave wallet set --name ag --private-key "$PRIVATE_KEY" 

enclave swarm up -v
