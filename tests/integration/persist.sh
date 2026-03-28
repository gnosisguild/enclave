#!/usr/bin/env bash

set -eu  # Exit immediately if a command exits with a non-zero status

# Get the directory of the currently executing script
THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Source the file from the same directory
source "$THIS_DIR/fns.sh"
source "$THIS_DIR/lib/utils.sh"

heading "Start the EVM node"

launch_evm

until curl -sf -X POST http://localhost:8545 -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' > /dev/null; do
  sleep 1
done

pnpm evm:clean
pnpm evm:deploy --network localhost

enclave_wallet_set cn1 "$PRIVATE_KEY_CN1"
enclave_wallet_set cn2 "$PRIVATE_KEY_CN2"
enclave_wallet_set cn3 "$PRIVATE_KEY_CN3"
enclave_wallet_set cn4 "$PRIVATE_KEY_CN4"
enclave_wallet_set cn5 "$PRIVATE_KEY_CN5"

heading "Setup ZK prover"
$ENCLAVE_BIN noir setup

# start swarm
enclave_nodes_up

waiton-files "$ROOT_DIR/target/debug/fake_encrypt"

heading "Add ciphernode $CIPHERNODE_ADDRESS_1"
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_1 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_2"
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_2 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_3"
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_3 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_4"
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_4 --network localhost

heading "Add ciphernode $CIPHERNODE_ADDRESS_5"
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_5 --network localhost

heading "Request Committee"

ENCODED_PARAMS=0x$($SCRIPT_DIR/lib/pack_e3_params.sh \
  --moduli 0xffffee001 \
  --moduli 0xffffc4001 \
  --degree 512 \
  --plaintext-modulus 100)

CURRENT_TIMESTAMP=$(get_evm_timestamp)
INPUT_WINDOW_START=$((CURRENT_TIMESTAMP + 20))
INPUT_WINDOW_END=$((CURRENT_TIMESTAMP + 30))

pnpm committee:new \
  --network localhost \
  --input-window-start "$INPUT_WINDOW_START" \
  --input-window-end "$INPUT_WINDOW_END" \
  --e3-params "$ENCODED_PARAMS" \
  --committee-size 0 \
  --proof-aggregation-enabled true

# Wait for any node's pubkey to signal committee finalization + DKG completion
waiton_any_pubkey

# Determine primary (rank=0) aggregator from on-chain committee ordering
PRIMARY_NODE=$(get_primary_committee_node 0)
echo "Primary committee node: $PRIMARY_NODE"
PRIMARY_PUBKEY_PATH=$(ciphernode_pubkey_path "$PRIMARY_NODE")
PRIMARY_PLAINTEXT_PATH=$(ciphernode_plaintext_path "$PRIMARY_NODE")

# restart the current primary node to exercise persistence on a regular ciphernode
enclave_nodes_stop "$PRIMARY_NODE"

sleep 8

enclave_nodes_start "$PRIMARY_NODE"

sleep 8

heading "Mock encrypted plaintext"
$SCRIPT_DIR/lib/fake_encrypt.sh --input "$PRIMARY_PUBKEY_PATH" --output "$SCRIPT_DIR/output/output.bin" --plaintext $PLAINTEXT --params "$ENCODED_PARAMS"

heading "Mock publish input e3-id"
pnpm e3-program:publishInput --network localhost  --e3-id 0 --data 0x12345678

sleep 6 # wait for input deadline to pass

waiton "$SCRIPT_DIR/output/output.bin"

heading "Publish ciphertext to EVM"
pnpm e3:publishCiphertext --e3-id 0 --network localhost --data-file "$SCRIPT_DIR/output/output.bin" --proof 0x12345678

waiton "$PRIMARY_PLAINTEXT_PATH"

ACTUAL=$(cut -d',' -f1,2 "$PRIMARY_PLAINTEXT_PATH")

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


gracefull_shutdown
