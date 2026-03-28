#!/usr/bin/env bash

# Restart/resilience integration tests.
#
# Tests different scenarios of killing and restarting nodes and the aggregator
# to verify the system recovers correctly.
#
# Scenarios:
#   1. Kill aggregator after key published, restart, complete E3
#   4. Kill aggregator during DKG (before key published), restart, DKG completes
#   5. Kill and restart a ciphernode mid-DKG, DKG completes
#   6. Kill all nodes and aggregator, restart all, complete E3
#
# Skipped:
#   2. Kill one ciphernode after key published (aggregator has no timeout for missing shares)
#   3. Kill aggregator during decryption (sync queries one peer which may not have all shares yet)

set -eu

THIS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

source "$THIS_DIR/fns.sh"
source "$THIS_DIR/lib/utils.sh"

E3_ID=0  # Incremented per committee:new call

# ── Common setup ──────────────────────────────────────────────────────────────

heading "Start the EVM node"

launch_evm

until curl -sf -X POST http://localhost:8545 -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' > /dev/null; do
  sleep 1
done

pnpm evm:clean
pnpm evm:deploy --network localhost

enclave_wallet_set ag "$PRIVATE_KEY_AG"
enclave_wallet_set cn1 "$PRIVATE_KEY_CN1"
enclave_wallet_set cn2 "$PRIVATE_KEY_CN2"
enclave_wallet_set cn3 "$PRIVATE_KEY_CN3"
enclave_wallet_set cn4 "$PRIVATE_KEY_CN4"
enclave_wallet_set cn5 "$PRIVATE_KEY_CN5"

heading "Setup ZK prover"
$ENCLAVE_BIN noir setup

ENCODED_PARAMS=0x$($SCRIPT_DIR/lib/pack_e3_params.sh \
  --moduli 0xffffee001 \
  --moduli 0xffffc4001 \
  --degree 512 \
  --plaintext-modulus 100)

# Register ciphernodes once (persists across scenarios via on-chain state)
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_1 --network localhost
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_2 --network localhost
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_3 --network localhost
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_4 --network localhost
pnpm ciphernode:add --ciphernode-address $CIPHERNODE_ADDRESS_5 --network localhost

# ── Helpers ───────────────────────────────────────────────────────────────────

clean_output() {
  rm -f "$SCRIPT_DIR/output/pubkey.bin" "$SCRIPT_DIR/output/output.bin" "$SCRIPT_DIR/output/plaintext.txt"
}

# Hard-stop all enclave processes between scenarios (keeps anvil running)
reset_nodes() {
  enclave_nodes_down
  sleep 2
  # Force-kill any leftover enclave processes from the previous scenario
  pkill -9 -f "target/debug/enclave" || true
  sleep 1
  clean_output
}

request_committee() {
  CURRENT_TIMESTAMP=$(get_evm_timestamp)
  INPUT_WINDOW_START=$((CURRENT_TIMESTAMP + 20))
  INPUT_WINDOW_END=$((CURRENT_TIMESTAMP + 30))

  pnpm committee:new \
    --network localhost \
    --input-window-start "$INPUT_WINDOW_START" \
    --input-window-end "$INPUT_WINDOW_END" \
    --e3-params "$ENCODED_PARAMS" \
    --committee-size 0 \
    --proof-aggregation-enabled false
}

publish_ciphertext() {
  local e3_id="$1"

  $SCRIPT_DIR/lib/fake_encrypt.sh \
    --input "$SCRIPT_DIR/output/pubkey.bin" \
    --output "$SCRIPT_DIR/output/output.bin" \
    --plaintext $PLAINTEXT \
    --params "$ENCODED_PARAMS"

  pnpm e3-program:publishInput --network localhost --e3-id "$e3_id" --data 0x12345678

  sleep 6

  waiton "$SCRIPT_DIR/output/output.bin"

  pnpm e3:publishCiphertext --e3-id "$e3_id" --network localhost \
    --data-file "$SCRIPT_DIR/output/output.bin" --proof 0x12345678
}

verify_plaintext() {
  local scenario="$1"

  waiton "$SCRIPT_DIR/output/plaintext.txt" 300

  ACTUAL=$(cut -d',' -f1,2 "$SCRIPT_DIR/output/plaintext.txt")

  if [[ "$ACTUAL" != "$PLAINTEXT"* ]]; then
    echo "Invalid plaintext decrypted: actual='$ACTUAL' expected='$PLAINTEXT'"
    echo "$scenario — FAILED"
    exit 1
  fi

  heading "$scenario — PASSED"
}

# ── Scenario 1: Kill aggregator after key published, restart ──────────────────

heading "Scenario 1: Aggregator restart after key published"

enclave_nodes_up
waiton-files "$ROOT_DIR/target/debug/fake_encrypt"
sleep 4

request_committee
waiton "$SCRIPT_DIR/output/pubkey.bin"

heading "Scenario 1: Killing aggregator"
enclave_nodes_stop ag
sleep 2

heading "Scenario 1: Restarting aggregator"
enclave_nodes_start ag
sleep 4

heading "Scenario 1: Publishing ciphertext after aggregator restart"
publish_ciphertext "$E3_ID"
verify_plaintext "Scenario 1: Aggregator restart after key published"

reset_nodes
E3_ID=$((E3_ID + 1))

# ── Scenario 4: Kill aggregator during DKG ────────────────────────────────────

heading "Scenario 4: Aggregator restart during DKG"

enclave_nodes_up
sleep 4

request_committee

# Kill aggregator immediately — DKG is in progress
heading "Scenario 4: Killing aggregator during DKG"
sleep 1
enclave_nodes_stop ag
sleep 2

heading "Scenario 4: Restarting aggregator"
enclave_nodes_start ag

# DKG should complete after aggregator restarts and syncs
waiton "$SCRIPT_DIR/output/pubkey.bin"

heading "Scenario 4: Publishing ciphertext"
publish_ciphertext "$E3_ID"
verify_plaintext "Scenario 4: Aggregator restart during DKG"

reset_nodes
E3_ID=$((E3_ID + 1))

# ── Scenario 5: Kill and restart ciphernode mid-DKG ───────────────────────────

heading "Scenario 5: Ciphernode restart mid-DKG"

enclave_nodes_up
sleep 4

request_committee

# Kill ciphernode during DKG
heading "Scenario 5: Killing ciphernode cn3 during DKG"
sleep 1
enclave_nodes_stop cn3
sleep 2

heading "Scenario 5: Restarting ciphernode cn3"
enclave_nodes_start cn3

# DKG should complete with cn3 back
waiton "$SCRIPT_DIR/output/pubkey.bin"

heading "Scenario 5: Publishing ciphertext"
publish_ciphertext "$E3_ID"
verify_plaintext "Scenario 5: Ciphernode restart mid-DKG"

reset_nodes
E3_ID=$((E3_ID + 1))

# ── Scenario 6: Kill all nodes, restart all ───────────────────────────────────

heading "Scenario 6: Full cluster restart"

enclave_nodes_up
sleep 4

request_committee
waiton "$SCRIPT_DIR/output/pubkey.bin"

heading "Scenario 6: Killing all nodes"
enclave_nodes_down
sleep 3

heading "Scenario 6: Restarting all nodes"
enclave_nodes_up
sleep 6

heading "Scenario 6: Publishing ciphertext after full restart"
publish_ciphertext "$E3_ID"
verify_plaintext "Scenario 6: Full cluster restart"

# ── Done ──────────────────────────────────────────────────────────────────────

gracefull_shutdown

heading "All restart tests PASSED !"
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
