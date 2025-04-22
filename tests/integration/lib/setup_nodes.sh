PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
CIPHERNODE_SECRET="This is secret"
NETWORK_PRIVATE_KEY_1="0x11a1e500a548b70d88184a1e042900c0ed6c57f8710bcc35dc8c85fa33d3f580"

rm -rf .enclave/

# Set the password for all ciphernodes
enclave password create --name cn1 --password "$CIPHERNODE_SECRET"
enclave password create --name cn2 --password "$CIPHERNODE_SECRET"
enclave password create --name cn3 --password "$CIPHERNODE_SECRET"
enclave password create --name cn4 --password "$CIPHERNODE_SECRET"
enclave password create --name ag --password "$CIPHERNODE_SECRET"
enclave wallet set --name ag --private-key "$PRIVATE_KEY"

# Set the network private key for all ciphernodes
enclave net generate-key --name cn1
enclave net generate-key --name cn2
enclave net generate-key --name cn3
enclave net generate-key --name cn4
enclave net generate-key --name ag
