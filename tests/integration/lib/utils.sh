# Get the current block timestamp from a local EVM node
# Usage: get_evm_timestamp [rpc_url]
get_evm_timestamp() {
  local rpc_url="${1:-http://localhost:8545}"
  curl -s -X POST "$rpc_url" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["latest",false],"id":1}' \
    | jq -r '.result.timestamp' | xargs printf "%d\n"
}
