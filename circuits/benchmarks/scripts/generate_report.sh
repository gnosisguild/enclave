#!/bin/bash

# generate_report.sh - Generates a markdown report from benchmark JSON results
# Usage: ./generate_report.sh --input-dir <dir> --output <file> --git-commit <hash> --git-branch <branch>

set -e

INPUT_DIR=""
OUTPUT_FILE=""
GIT_COMMIT="unknown"
GIT_BRANCH="unknown"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --input-dir)
            INPUT_DIR="$2"
            shift 2
            ;;
        --output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        --git-commit)
            GIT_COMMIT="$2"
            shift 2
            ;;
        --git-branch)
            GIT_BRANCH="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

if [ -z "$INPUT_DIR" ] || [ -z "$OUTPUT_FILE" ]; then
    echo "Usage: $0 --input-dir <dir> --output <file> [--git-commit <hash>] [--git-branch <branch>]"
    exit 1
fi

# Helper functions
format_bytes() {
    local bytes=$1
    if [ "$bytes" -eq 0 ]; then
        echo "0 B"
    elif [ "$bytes" -lt 1024 ]; then
        echo "${bytes} B"
    elif [ "$bytes" -lt 1048576 ]; then
        local kb=$(echo "scale=5; $bytes/1024" | bc | awk '{printf "%.2f", $0}')
        echo "${kb} KB"
    else
        local mb=$(echo "scale=5; $bytes/1048576" | bc | awk '{printf "%.2f", $0}')
        echo "${mb} MB"
    fi
}

format_time() {
    local seconds=$1
    # Format to 2 decimal places
    local s=$(echo "$seconds" | awk '{printf "%.2f", $0}')
    echo "${s} s"
}

format_gates() {
    local gates=$1
    if [ "$gates" -ge 1000000 ]; then
        local m=$(echo "scale=5; $gates/1000000" | bc | awk '{printf "%.2f", $0}')
        echo "${m}M"
    elif [ "$gates" -ge 1000 ]; then
        local k=$(echo "scale=5; $gates/1000" | bc | awk '{printf "%.2f", $0}')
        echo "${k}K"
    else
        echo "$gates"
    fi
}


# Helper: return "dkg" or "threshold" from circuit_path in JSON
category_of() {
    local path
    path=$(jq -r '.circuit_path' "$1")
    if [[ "$path" == *"/dkg/"* ]]; then
        echo "dkg"
    elif [[ "$path" == *"/threshold/"* ]]; then
        echo "threshold"
    else
        echo "other"
    fi
}

# Start building report
TIMESTAMP=$(date -u "+%Y-%m-%d %H:%M:%S UTC")

cat > "$OUTPUT_FILE" << EOF
# Enclave ZK Circuit Benchmarks

**Generated:** ${TIMESTAMP}

**Git Branch:** \`${GIT_BRANCH}\`  
**Git Commit:** \`${GIT_COMMIT}\`

---

## Summary

### DKG

#### Timing Metrics

| Circuit | Compile | Execute | Prove | Verify |
|---------|---------|---------|-------|--------|
EOF

for json_file in "$INPUT_DIR"/*.json; do
    [ -f "$json_file" ] || continue
    [ "$(category_of "$json_file")" = "dkg" ] || continue
    circuit=$(jq -r '.circuit_name' "$json_file")
    compile_time=$(jq -r '.compilation.time_seconds' "$json_file")
    execute_time=$(jq -r '.execution.time_seconds' "$json_file")
    prove_time=$(jq -r '.proof_generation.time_seconds' "$json_file")
    verify_time=$(jq -r '.verification.time_seconds' "$json_file")
    compile_fmt=$(format_time "$compile_time")
    execute_fmt=$(format_time "$execute_time")
    prove_fmt=$(format_time "$prove_time")
    verify_fmt=$(format_time "$verify_time")
    echo "| $circuit | $compile_fmt | $execute_fmt | $prove_fmt | $verify_fmt |" >> "$OUTPUT_FILE"
done

cat >> "$OUTPUT_FILE" << EOF

#### Size & Circuit Metrics

| Circuit | Opcodes | Gates | Circuit Size | Witness | VK Size | Proof Size |
|---------|---------|-------|--------------|---------|---------|------------|
EOF

for json_file in "$INPUT_DIR"/*.json; do
    [ -f "$json_file" ] || continue
    [ "$(category_of "$json_file")" = "dkg" ] || continue
    circuit=$(jq -r '.circuit_name' "$json_file")
    opcodes=$(jq -r '.gates.acir_opcodes // 0' "$json_file")
    gates=$(jq -r '.gates.total_gates' "$json_file")
    circuit_size=$(jq -r '.compilation.circuit_size_bytes' "$json_file")
    witness_size=$(jq -r '.execution.witness_size_bytes' "$json_file")
    vk_size=$(jq -r '.vk_generation.vk_size_bytes' "$json_file")
    proof_size=$(jq -r '.proof_generation.proof_size_bytes' "$json_file")
    gates_fmt=$(format_gates "$gates")
    circuit_size_fmt=$(format_bytes "$circuit_size")
    witness_size_fmt=$(format_bytes "$witness_size")
    vk_size_fmt=$(format_bytes "$vk_size")
    proof_size_fmt=$(format_bytes "$proof_size")
    echo "| $circuit | $opcodes | $gates_fmt | $circuit_size_fmt | $witness_size_fmt | $vk_size_fmt | $proof_size_fmt |" >> "$OUTPUT_FILE"
done

cat >> "$OUTPUT_FILE" << EOF

### Threshold

#### Timing Metrics

| Circuit | Compile | Execute | Prove | Verify |
|---------|---------|---------|-------|--------|
EOF

for json_file in "$INPUT_DIR"/*.json; do
    [ -f "$json_file" ] || continue
    [ "$(category_of "$json_file")" = "threshold" ] || continue
    circuit=$(jq -r '.circuit_name' "$json_file")
    compile_time=$(jq -r '.compilation.time_seconds' "$json_file")
    execute_time=$(jq -r '.execution.time_seconds' "$json_file")
    prove_time=$(jq -r '.proof_generation.time_seconds' "$json_file")
    verify_time=$(jq -r '.verification.time_seconds' "$json_file")
    compile_fmt=$(format_time "$compile_time")
    execute_fmt=$(format_time "$execute_time")
    prove_fmt=$(format_time "$prove_time")
    verify_fmt=$(format_time "$verify_time")
    echo "| $circuit | $compile_fmt | $execute_fmt | $prove_fmt | $verify_fmt |" >> "$OUTPUT_FILE"
done

cat >> "$OUTPUT_FILE" << EOF

#### Size & Circuit Metrics

| Circuit | Opcodes | Gates | Circuit Size | Witness | VK Size | Proof Size |
|---------|---------|-------|--------------|---------|---------|------------|
EOF

for json_file in "$INPUT_DIR"/*.json; do
    [ -f "$json_file" ] || continue
    [ "$(category_of "$json_file")" = "threshold" ] || continue
    circuit=$(jq -r '.circuit_name' "$json_file")
    opcodes=$(jq -r '.gates.acir_opcodes // 0' "$json_file")
    gates=$(jq -r '.gates.total_gates' "$json_file")
    circuit_size=$(jq -r '.compilation.circuit_size_bytes' "$json_file")
    witness_size=$(jq -r '.execution.witness_size_bytes' "$json_file")
    vk_size=$(jq -r '.vk_generation.vk_size_bytes' "$json_file")
    proof_size=$(jq -r '.proof_generation.proof_size_bytes' "$json_file")
    gates_fmt=$(format_gates "$gates")
    circuit_size_fmt=$(format_bytes "$circuit_size")
    witness_size_fmt=$(format_bytes "$witness_size")
    vk_size_fmt=$(format_bytes "$vk_size")
    proof_size_fmt=$(format_bytes "$proof_size")
    echo "| $circuit | $opcodes | $gates_fmt | $circuit_size_fmt | $witness_size_fmt | $vk_size_fmt | $proof_size_fmt |" >> "$OUTPUT_FILE"
done

# Detailed metrics by circuit, grouped by DKG / Threshold
cat >> "$OUTPUT_FILE" << EOF

## Circuit Details

EOF

for category in dkg threshold; do
    title="DKG"; [ "$category" = "threshold" ] && title="Threshold"
    echo "### $title" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    circuits=$(for json_file in "$INPUT_DIR"/*.json; do
        [ -f "$json_file" ] || continue
        [ "$(category_of "$json_file")" = "$category" ] || continue
        jq -r '.circuit_name' "$json_file"
    done | sort -u)
    for circuit in $circuits; do
        json_file=""
        for f in "$INPUT_DIR"/*.json; do
            [ -f "$f" ] || continue
            [ "$(category_of "$f")" = "$category" ] || continue
            c=$(jq -r '.circuit_name' "$f")
            if [ "$c" = "$circuit" ]; then
                json_file="$f"
                break
            fi
        done
        [ -z "$json_file" ] && continue
        echo "#### $circuit" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        compile=$(jq -r '.compilation.time_seconds' "$json_file")
        execute=$(jq -r '.execution.time_seconds' "$json_file")
        opcodes=$(jq -r '.gates.acir_opcodes // 0' "$json_file")
        gates=$(jq -r '.gates.total_gates' "$json_file")
        vk_gen=$(jq -r '.vk_generation.time_seconds' "$json_file")
        prove=$(jq -r '.proof_generation.time_seconds' "$json_file")
        verify=$(jq -r '.verification.time_seconds' "$json_file")
        circuit_size=$(jq -r '.compilation.circuit_size_bytes' "$json_file")
        witness_size=$(jq -r '.execution.witness_size_bytes' "$json_file")
        vk_size=$(jq -r '.vk_generation.vk_size_bytes' "$json_file")
        proof_size=$(jq -r '.proof_generation.proof_size_bytes' "$json_file")
        cat >> "$OUTPUT_FILE" << INNER
| Metric | Value |
|--------|-------|
| **Compilation** | $(format_time $compile) |
| **Execution** | $(format_time $execute) |
| **VK Generation** | $(format_time $vk_gen) |
| **Proof Generation** | $(format_time $prove) |
| **Verification** | $(format_time $verify) |
| **ACIR Opcodes** | $opcodes |
| **Total Gates** | $gates |
| **Circuit Size** | $(format_bytes $circuit_size) |
| **Witness Size** | $(format_bytes $witness_size) |
| **VK Size** | $(format_bytes $vk_size) |
| **Proof Size** | $(format_bytes $proof_size) |

INNER
    done
    echo "" >> "$OUTPUT_FILE"
done

# System info (from first JSON file)
first_json=$(ls "$INPUT_DIR"/*.json 2>/dev/null | head -1)
if [ -n "$first_json" ]; then
    cat >> "$OUTPUT_FILE" << EOF
## System Information

### Hardware

EOF
    
    cpu_model=$(jq -r '.system_info.cpu_model // "unknown"' "$first_json")
    cpu_cores=$(jq -r '.system_info.cpu_cores // "unknown"' "$first_json")
    ram_gb=$(jq -r '.system_info.ram_gb // "unknown"' "$first_json")
    os=$(jq -r '.system_info.os' "$first_json")
    arch=$(jq -r '.system_info.arch' "$first_json")
    
    echo "- **CPU:** $cpu_model" >> "$OUTPUT_FILE"
    echo "- **CPU Cores:** $cpu_cores" >> "$OUTPUT_FILE"
    echo "- **RAM:** ${ram_gb} GB" >> "$OUTPUT_FILE"
    echo "- **OS:** $os" >> "$OUTPUT_FILE"
    echo "- **Architecture:** $arch" >> "$OUTPUT_FILE"
    
    cat >> "$OUTPUT_FILE" << EOF

### Software

EOF
    
    nargo=$(jq -r '.system_info.nargo_version' "$first_json")
    bb=$(jq -r '.system_info.bb_version' "$first_json")
    
    echo "- **Nargo Version:** $nargo" >> "$OUTPUT_FILE"
    echo "- **Barretenberg Version:** $bb" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
fi

echo "✓ Report generated successfully: $OUTPUT_FILE"
