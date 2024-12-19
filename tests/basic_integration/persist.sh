#!/usr/bin/env bash

set -eu  # Exit immediately if a command exits with a non-zero status

# Get the directory of the currently executing script
THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Source the file from the same directory
source "$THIS_DIR/fns.sh"

heading "Start the EVM node"

launch_evm

until curl -f -s "http://localhost:8545" > /dev/null; do
  sleep 1
done

# Set the password for all ciphernodes
set_password cn1 "$CIPHERNODE_SECRET"
set_password cn2 "$CIPHERNODE_SECRET"
set_password cn3 "$CIPHERNODE_SECRET"
set_password cn4 "$CIPHERNODE_SECRET"
set_password ag "$CIPHERNODE_SECRET"
set_private_key ag "$PRIVATE_KEY"

# Set the network private key for all ciphernodes
set_network_private_key cn1 "$NETWORK_PRIVATE_KEY_1"
set_network_private_key cn2 "$NETWORK_PRIVATE_KEY_2"
set_network_private_key cn3 "$NETWORK_PRIVATE_KEY_3"
set_network_private_key cn4 "$NETWORK_PRIVATE_KEY_4"
set_network_private_key ag "$NETWORK_PRIVATE_KEY_AG"

# Launch 4 ciphernodes
launch_ciphernode cn1
launch_ciphernode cn2
launch_ciphernode cn3
launch_ciphernode cn4
launch_aggregator ag

waiton-files "$ROOT_DIR/packages/ciphernode/target/debug/enclave" "$ROOT_DIR/packages/ciphernode/target/debug/fake_encrypt"

heading "Add ciphernode $CIPHERNODE_ADDRESS_1"
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_1 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_2"
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_2 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_3"
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_3 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_4"
yarn ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_4 --network localhost

heading "Request Committee"

ENCODED_PARAMS=0x$($SCRIPT_DIR/lib/pack_e3_params.sh --moduli 0x3FFFFFFF000001 --degree 2048 --plaintext-modulus 1032193)

yarn committee:new --network localhost --duration 4 --e3-params "$ENCODED_PARAMS"

waiton "$SCRIPT_DIR/output/pubkey.bin"
PUBLIC_KEY=$(xxd -p -c 10000000 "$SCRIPT_DIR/output/pubkey.bin")


# kill aggregator
kill_proc ag

sleep 2

# relaunch the aggregator
launch_aggregator ag

sleep 2

heading "Mock encrypted plaintext"
$SCRIPT_DIR/lib/fake_encrypt.sh --input "$SCRIPT_DIR/output/pubkey.bin" --output "$SCRIPT_DIR/output/output.bin" --plaintext $PLAINTEXT

heading "Mock activate e3-id"
# NOTE using -s to avoid key spaming the output
yarn -s e3:activate --e3-id 0 --public-key "0x$PUBLIC_KEY" --network localhost

heading "Mock publish input e3-id"
yarn e3:publishInput --network localhost  --e3-id 0 --data 0x12345678

sleep 4 # wait for input deadline to pass

waiton "$SCRIPT_DIR/output/output.bin"

heading "Publish ciphertext to EVM"
yarn e3:publishCiphertext --e3-id 0 --network localhost --data-file "$SCRIPT_DIR/output/output.bin" --proof 0x12345678

waiton "$SCRIPT_DIR/output/plaintext.txt"

ACTUAL=$(cat $SCRIPT_DIR/output/plaintext.txt)
 

# Assume plaintext is shorter

if [[ "$ACTUAL" != "$PLAINTEXT"* ]]; then
  echo "Invalid plaintext decrypted: actual='$ACTUAL' expected='$PLAINTEXT'"
  echo "Test FAILED"
  exit 1
fi

heading "Test PASSED !"
echo -e "\033[32m                                                              
                                               ██████         
                                             ██████           
                                           ██████             
                                         ██████               
                                       ██████                 
                                     ██████                   
                       ██          ██████                     
                       ████      ██████                       
                       ██████  ██████                         
                        ██████████                            
                         ████████                             
                          ██████                              
                           ████                               
                            ██                                
                                                              \033[0m"

pkill -15 -f "target/debug/enclave" || true
pkill -15 -f "target/debug/aggregator" || true

sleep 4

cleanup 0

