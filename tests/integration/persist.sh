#!/usr/bin/env bash

set -eu  # Exit immediately if a command exits with a non-zero status

# Get the directory of the currently executing script
THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Source the file from the same directory
source "$THIS_DIR/fns.sh"

time {
  heading "Start the EVM node"

  launch_evm

  until curl -f -s "http://localhost:8545" > /dev/null; do
    sleep 1
  done

  # set wallet to ag specifically
  enclave_wallet_set ag "$PRIVATE_KEY"

  # start swarm
  enclave_nodes_up

  waiton-files "$ROOT_DIR/target/debug/fake_encrypt"
  timefooter
}

time {
  heading "Add ciphernode $CIPHERNODE_ADDRESS_1"
  pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_1 --network localhost
  timefooter
}

time {
  heading "Add ciphernode $CIPHERNODE_ADDRESS_2"
  pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_2 --network localhost
  timefooter
}

time {
  heading "Add ciphernode $CIPHERNODE_ADDRESS_3"
  pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_3 --network localhost
  timefooter
}

time {
  heading "Add ciphernode $CIPHERNODE_ADDRESS_4"
  pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_4 --network localhost
  timefooter
}

time {
  heading "Request Committee"

  ENCODED_PARAMS=0x$($SCRIPT_DIR/lib/pack_e3_params.sh --moduli 0x3FFFFFFF000001 --degree 2048 --plaintext-modulus 1032193)

  pnpm committee:new --network localhost --duration 4 --e3-params "$ENCODED_PARAMS"

  waiton "$SCRIPT_DIR/output/pubkey.bin"
  PUBLIC_KEY=$(xxd -p -c 10000000 "$SCRIPT_DIR/output/pubkey.bin")


  # kill aggregator
  enclave_nodes_stop ag

  sleep 2

  # relaunch the aggregator
  enclave_nodes_start ag

  sleep 2
  timefooter
}

time {
  heading "Mock encrypted plaintext"
  $SCRIPT_DIR/lib/fake_encrypt.sh --input "$SCRIPT_DIR/output/pubkey.bin" --output "$SCRIPT_DIR/output/output.bin" --plaintext $PLAINTEXT
  timefooter
}

time {
  heading "Mock activate e3-id"
  # NOTE using -s to avoid key spaming the output
  pnpm -s e3:activate --e3-id 0 --public-key "0x$PUBLIC_KEY" --network localhost
  timefooter
}

time {
  heading "Mock publish input e3-id"
  pnpm e3:publishInput --network localhost  --e3-id 0 --data 0x12345678

  sleep 4 # wait for input deadline to pass

  waiton "$SCRIPT_DIR/output/output.bin"
  timefooter
}

time {
  heading "Publish ciphertext to EVM"
  pnpm e3:publishCiphertext --e3-id 0 --network localhost --data-file "$SCRIPT_DIR/output/output.bin" --proof 0x12345678

  waiton "$SCRIPT_DIR/output/plaintext.txt"
  timefooter
}

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


gracefull_shutdown

