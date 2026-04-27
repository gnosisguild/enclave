#!/bin/bash

set -e

INPUT_DIR=""
OUTPUT_FILE=""
GIT_COMMIT="unknown"
GIT_BRANCH="unknown"
GAS_JSON=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --input-dir) INPUT_DIR="$2"; shift 2 ;;
        --output) OUTPUT_FILE="$2"; shift 2 ;;
        --git-commit) GIT_COMMIT="$2"; shift 2 ;;
        --git-branch) GIT_BRANCH="$2"; shift 2 ;;
        --gas-json) GAS_JSON="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [ -z "$INPUT_DIR" ] || [ -z "$OUTPUT_FILE" ]; then
    echo "Usage: $0 --input-dir <dir> --output <file> [--git-commit <hash>] [--git-branch <branch>] [--gas-json <file>]"
    exit 1
fi

format_s() { awk -v v="$1" 'BEGIN{printf "%.2f", v}'; }
format_ms() { echo "$1 * 1000" | bc -l | awk '{printf "%.2f", $0}'; }
format_kb() { echo "$1 / 1024" | bc -l | awk '{printf "%.2f", $0}'; }

find_json_by_path_fragment() {
    local frag="$1"
    for json_file in "$INPUT_DIR"/*.json; do
        [ -f "$json_file" ] || continue
        local circuit_path
        circuit_path=$(jq -r '.circuit_path // ""' "$json_file")
        if [[ "$circuit_path" == *"$frag"* ]]; then
            echo "$json_file"
            return
        fi
    done
    echo ""
}

emit_circuit_row() {
    local label="$1"
    local path_fragment="$2"
    local json_file
    json_file=$(find_json_by_path_fragment "$path_fragment")
    if [ -z "$json_file" ]; then
        echo "| $label | N/A | N/A | N/A | N/A |" >> "$OUTPUT_FILE"
        return
    fi
    local constraints prove verify proof_size
    constraints=$(jq -r '.gates.total_gates // 0' "$json_file")
    prove=$(jq -r '.proof_generation.time_seconds // 0' "$json_file")
    verify=$(jq -r '.verification.time_seconds // 0' "$json_file")
    proof_size=$(jq -r '.proof_generation.proof_size_bytes // 0' "$json_file")
    echo "| $label | $constraints | $(format_s "$prove") | $(format_ms "$verify") | $(format_kb "$proof_size") |" >> "$OUTPUT_FILE"
}

emit_user_data_enc_row() {
    local wrapper ct0 ct1
    wrapper=$(find_json_by_path_fragment "/threshold/user_data_encryption")
    ct0=$(find_json_by_path_fragment "/threshold/user_data_encryption_ct0")
    ct1=$(find_json_by_path_fragment "/threshold/user_data_encryption_ct1")

    if [ -n "$wrapper" ]; then
        local constraints prove verify proof_size
        constraints=$(jq -r '.gates.total_gates // 0' "$wrapper")
        prove=$(jq -r '.proof_generation.time_seconds // 0' "$wrapper")
        verify=$(jq -r '.verification.time_seconds // 0' "$wrapper")
        proof_size=$(jq -r '.proof_generation.proof_size_bytes // 0' "$wrapper")
        if [ "$proof_size" -gt 0 ]; then
            echo "| user_data_encryption | $constraints | $(format_s "$prove") | $(format_ms "$verify") | $(format_kb "$proof_size") |" >> "$OUTPUT_FILE"
            return
        fi
    fi

    if [ -n "$ct0" ] && [ -n "$ct1" ]; then
        local constraints prove verify proof_size
        constraints=$(echo "$(jq -r '.gates.total_gates // 0' "$ct0") + $(jq -r '.gates.total_gates // 0' "$ct1")" | bc)
        prove=$(echo "$(jq -r '.proof_generation.time_seconds // 0' "$ct0") + $(jq -r '.proof_generation.time_seconds // 0' "$ct1")" | bc -l)
        verify=$(echo "$(jq -r '.verification.time_seconds // 0' "$ct0") + $(jq -r '.verification.time_seconds // 0' "$ct1")" | bc -l)
        proof_size=$(echo "$(jq -r '.proof_generation.proof_size_bytes // 0' "$ct0") + $(jq -r '.proof_generation.proof_size_bytes // 0' "$ct1")" | bc)
        echo "| user_data_encryption | $constraints | $(format_s "$prove") | $(format_ms "$verify") | $(format_kb "$proof_size") |" >> "$OUTPUT_FILE"
        return
    fi

    echo "| user_data_encryption | N/A | N/A | N/A | N/A |" >> "$OUTPUT_FILE"
}

verify_gas_for_artifact() {
    local artifact="$1"
    [ -f "$GAS_JSON" ] || { echo "N/A"; return; }
    local key=""
    case "$artifact" in
        Π_DKG) key="dkg" ;;
        Π_user) key="user" ;;
        Π_dec) key="dec" ;;
        *) echo "N/A"; return ;;
    esac
    local value
    value=$(jq -r ".verify_gas.${key} // empty" "$GAS_JSON")
    if [ -z "$value" ] || [ "$value" = "null" ]; then
        echo "N/A"
    else
        echo "$value"
    fi
}

artifact_metrics() {
    local name="$1"
    local label="$2"
    local verify_gas="$3"

    if [ "$label" = "user_data_encryption" ]; then
        local ct0 ct1
        ct0=$(find_json_by_path_fragment "/threshold/user_data_encryption_ct0")
        ct1=$(find_json_by_path_fragment "/threshold/user_data_encryption_ct1")
        if [ -z "$ct0" ] || [ -z "$ct1" ]; then
            echo "| $name | N/A | N/A | $verify_gas | N/A | N/A |" >> "$OUTPUT_FILE"
            return
        fi
        local proof_size public_size calldata total
        proof_size=$(echo "$(jq -r '.proof_generation.proof_size_bytes // 0' "$ct0") + $(jq -r '.proof_generation.proof_size_bytes // 0' "$ct1")" | bc)
        public_size=$(echo "$(jq -r '.verification.public_inputs_size_bytes // 0' "$ct0") + $(jq -r '.verification.public_inputs_size_bytes // 0' "$ct1")" | bc)
        calldata=$(echo "$(jq -r '.verification.calldata_gas_total // 0' "$ct0") + $(jq -r '.verification.calldata_gas_total // 0' "$ct1")" | bc)
        total="N/A"
        if [ "$verify_gas" != "N/A" ]; then total=$((verify_gas + calldata)); fi
        echo "| $name | $(format_kb "$proof_size") KB | $(format_kb "$public_size") KB | $verify_gas | $calldata | $total |" >> "$OUTPUT_FILE"
        return
    fi

    local json_file
    json_file=$(find_json_by_path_fragment "$label")
    if [ -z "$json_file" ]; then
        echo "| $name | N/A | N/A | $verify_gas | N/A | N/A |" >> "$OUTPUT_FILE"
        return
    fi
    local proof_size public_size calldata total
    proof_size=$(jq -r '.proof_generation.proof_size_bytes // 0' "$json_file")
    public_size=$(jq -r '.verification.public_inputs_size_bytes // 0' "$json_file")
    calldata=$(jq -r '.verification.calldata_gas_total // 0' "$json_file")
    total="N/A"
    if [ "$verify_gas" != "N/A" ]; then total=$((verify_gas + calldata)); fi
    echo "| $name | $(format_kb "$proof_size") KB | $(format_kb "$public_size") KB | $verify_gas | $calldata | $total |" >> "$OUTPUT_FILE"
}

sum_phase_metrics() {
    local labels="$1"
    local prove_sum="0"
    local proof_sum="0"
    local bandwidth_sum="0"
    local count=0
    for label in $labels; do
        local jf
        jf=$(find_json_by_path_fragment "$label")
        [ -n "$jf" ] || continue
        local p ps pub
        p=$(jq -r '.proof_generation.time_seconds // 0' "$jf")
        ps=$(jq -r '.proof_generation.proof_size_bytes // 0' "$jf")
        pub=$(jq -r '.verification.public_inputs_size_bytes // 0' "$jf")
        prove_sum=$(echo "$prove_sum + $p" | bc -l)
        proof_sum=$(echo "$proof_sum + $ps" | bc -l)
        bandwidth_sum=$(echo "$bandwidth_sum + $ps + $pub" | bc -l)
        count=$((count + 1))
    done
    if [ "$count" -eq 0 ]; then echo "N/A|N/A|N/A"; return; fi
    echo "$(format_s "$prove_sum") s|$(format_kb "$proof_sum") KB|$(format_kb "$bandwidth_sum") KB"
}

TIMESTAMP=$(date -u "+%Y-%m-%d %H:%M:%S UTC")

cat > "$OUTPUT_FILE" <<EOF
# Enclave ZK Circuit Benchmarks

**Generated:** ${TIMESTAMP}

**Git Branch:** \`${GIT_BRANCH}\`  
**Git Commit:** \`${GIT_COMMIT}\`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
|---------|-------------|----------------|------------------|-----------------|
EOF

emit_circuit_row "C0" "/dkg/pk"
emit_circuit_row "C1" "/threshold/pk_generation"
emit_circuit_row "C2a" "/dkg/sk_share_computation"
emit_circuit_row "C2b" "/dkg/e_sm_share_computation"
emit_circuit_row "C3a" "/dkg/share_encryption"
emit_circuit_row "C3b" "/dkg/share_encryption"
emit_circuit_row "C4a" "/dkg/share_decryption"
emit_circuit_row "C4b" "/dkg/share_decryption"
emit_circuit_row "C5" "/threshold/pk_aggregation"
emit_user_data_enc_row
emit_circuit_row "C6" "/threshold/share_decryption"
emit_circuit_row "C7" "/threshold/decrypted_shares_aggregation"

cat >> "$OUTPUT_FILE" <<EOF

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
|----------|------------|-------------------|------------|--------------|-----------|
EOF

artifact_metrics "Π_DKG" "/threshold/pk_aggregation" "$(verify_gas_for_artifact Π_DKG)"
artifact_metrics "Π_user" "user_data_encryption" "$(verify_gas_for_artifact Π_user)"
artifact_metrics "Π_dec" "/threshold/decrypted_shares_aggregation" "$(verify_gas_for_artifact Π_dec)"

p1=$(sum_phase_metrics "/dkg/pk /threshold/pk_generation /dkg/sk_share_computation /dkg/e_sm_share_computation /dkg/share_encryption /dkg/share_decryption")
p2=$(sum_phase_metrics "/threshold/pk_aggregation")
p3=$(sum_phase_metrics "/threshold/user_data_encryption_ct0 /threshold/user_data_encryption_ct1")
p4n=$(sum_phase_metrics "/threshold/share_decryption")
p4a=$(sum_phase_metrics "/threshold/decrypted_shares_aggregation")
IFS='|' read -r p1t p1s p1b <<< "$p1"
IFS='|' read -r p2t p2s p2b <<< "$p2"
IFS='|' read -r p3t p3s p3b <<< "$p3"
IFS='|' read -r p4nt p4ns p4nb <<< "$p4n"
IFS='|' read -r p4at p4as p4ab <<< "$p4a"

cat >> "$OUTPUT_FILE" <<EOF

### Role / Phase / Activity

| Role | Phase | Activity | Prove time | Proof size | Bandwidth |
|------|-------|----------|------------|------------|-----------|
| Each ciphernode | P1 | one-time DKG participation | $p1t | $p1s | $p1b |
| Aggregator | P2 | combine folds + C5 | $p2t | $p2s | $p2b |
| User | P3 | per user input | $p3t | $p3s | $p3b |
| Each ciphernode | P4 | per computation output (C6) | $p4nt | $p4ns | $p4nb |
| Aggregator | P4 | per computation output (C7+fold) | $p4at | $p4as | $p4ab |
EOF

first_json=$(ls "$INPUT_DIR"/*.json 2>/dev/null | head -1)
if [ -n "$first_json" ]; then
    cpu_model=$(jq -r '.system_info.cpu_model // "unknown"' "$first_json")
    cpu_cores=$(jq -r '.system_info.cpu_cores // "unknown"' "$first_json")
    ram_gb=$(jq -r '.system_info.ram_gb // "unknown"' "$first_json")
    os=$(jq -r '.system_info.os // "unknown"' "$first_json")
    arch=$(jq -r '.system_info.arch // "unknown"' "$first_json")
    nargo=$(jq -r '.system_info.nargo_version // "unknown"' "$first_json")
    bb=$(jq -r '.system_info.bb_version // "unknown"' "$first_json")
    cat >> "$OUTPUT_FILE" <<EOF

## System Information

### Hardware
- **CPU:** $cpu_model
- **CPU Cores:** $cpu_cores
- **RAM:** ${ram_gb} GB
- **OS:** $os
- **Architecture:** $arch

### Software
- **Nargo Version:** $nargo
- **Barretenberg Version:** $bb
EOF
fi

echo "✓ Report generated successfully: $OUTPUT_FILE"
