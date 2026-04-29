#!/bin/bash

set -e

INPUT_DIR=""
OUTPUT_FILE=""
GIT_COMMIT="unknown"
GIT_BRANCH="unknown"
GAS_JSON=""
# Optional JSON from `BENCHMARK_SUMMARY_OUTPUT` (same schema as embedded `integration_summary` in
# crisp_verify_gas.json). Used when gas JSON has null/broken integration_summary but timings exist
# on disk (e.g. long secure run wrote /tmp/summary_secure.json separately).
INTEGRATION_SUMMARY_FILE=""
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCHMARKS_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(cd "${BENCHMARKS_DIR}/../.." && pwd)"

while [[ $# -gt 0 ]]; do
    case $1 in
        --input-dir) INPUT_DIR="$2"; shift 2 ;;
        --output) OUTPUT_FILE="$2"; shift 2 ;;
        --git-commit) GIT_COMMIT="$2"; shift 2 ;;
        --git-branch) GIT_BRANCH="$2"; shift 2 ;;
        --gas-json) GAS_JSON="$2"; shift 2 ;;
        --integration-summary) INTEGRATION_SUMMARY_FILE="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [ -z "$INPUT_DIR" ] || [ -z "$OUTPUT_FILE" ]; then
    echo "Usage: $0 --input-dir <dir> --output <file> [--git-commit <hash>] [--git-branch <branch>] [--gas-json <file>] [--integration-summary <timings.json>]"
    exit 1
fi

format_s() { awk -v v="$1" 'BEGIN{printf "%.2f", v}'; }
format_ms() { echo "$1 * 1000" | bc -l | awk '{printf "%.2f", $0}'; }
format_kb() { echo "$1 / 1024" | bc -l | awk '{printf "%.2f", $0}'; }

hex_len_bytes() {
    local hex="${1:-}"
    python3 - "$hex" <<'PY'
import sys
h = sys.argv[1] or ""
if h.startswith("0x"):
    h = h[2:]
if len(h) % 2 != 0:
    print("0")
else:
    print(len(h) // 2)
PY
}

calldata_gas_from_hex() {
    local hex="${1:-}"
    python3 - "$hex" <<'PY'
import sys
h = sys.argv[1] or ""
if h.startswith("0x"):
    h = h[2:]
if len(h) % 2 != 0:
    print("0")
    raise SystemExit(0)
gas = 0
for i in range(0, len(h), 2):
    b = h[i : i + 2]
    gas += 4 if b == "00" else 16
print(gas)
PY
}

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
    local artifact_key=""
    case "$name" in
        Π_DKG) artifact_key="dkg" ;;
        Π_dec) artifact_key="dec" ;;
    esac

    if [ "$label" = "user_data_encryption" ]; then
        local wrapper
        wrapper=$(find_json_by_path_fragment "/threshold/user_data_encryption")
        local proof_size public_size calldata total

        # Prefer the wrapper artifact when available; it matches what is posted/verified on-chain.
        if [ -n "$wrapper" ]; then
            proof_size=$(jq -r '.proof_generation.proof_size_bytes // 0' "$wrapper")
            public_size=$(jq -r '.verification.public_inputs_size_bytes // 0' "$wrapper")
            calldata=$(jq -r '.verification.calldata_gas_total // 0' "$wrapper")
            total="N/A"
            if [ "$verify_gas" != "N/A" ]; then total=$((verify_gas + calldata)); fi
            echo "| $name | $(format_kb "$proof_size") KB | $(format_kb "$public_size") KB | $verify_gas | $calldata | $total |" >> "$OUTPUT_FILE"
            return
        fi

        echo "| $name | N/A | N/A | $verify_gas | N/A | N/A |" >> "$OUTPUT_FILE"
        return
    fi

    # Prefer folded artifact sizes/calldata (when present) so table aligns with folded verify gas.
    if [ -n "$artifact_key" ] && [ -f "$GAS_JSON" ]; then
        local folded_proof_size folded_public_size folded_calldata
        folded_proof_size=$(jq -r ".artifact_sizes_bytes.${artifact_key}.proof // empty" "$GAS_JSON")
        folded_public_size=$(jq -r ".artifact_sizes_bytes.${artifact_key}.public_inputs // empty" "$GAS_JSON")
        folded_calldata=$(jq -r ".calldata_gas.${artifact_key}.total // empty" "$GAS_JSON")
        if [ -n "$folded_proof_size" ] && [ "$folded_proof_size" != "null" ] && [ "$folded_proof_size" != "0" ] \
            && [ -n "$folded_public_size" ] && [ "$folded_public_size" != "null" ] && [ "$folded_public_size" != "0" ] \
            && [ -n "$folded_calldata" ] && [ "$folded_calldata" != "null" ] && [ "$folded_calldata" != "0" ]; then
            local folded_total="N/A"
            if [ "$verify_gas" != "N/A" ]; then folded_total=$((verify_gas + folded_calldata)); fi
            echo "| $name | $(format_kb "$folded_proof_size") KB | $(format_kb "$folded_public_size") KB | $verify_gas | $folded_calldata | $folded_total |" >> "$OUTPUT_FILE"
            return
        fi
    fi

    # Sizes + calldata from integration summary folded hex (when gas JSON has no folded export).
    if [ -n "$artifact_key" ] && [ -n "$INTEGRATION_SUMMARY_FILE" ] && [ -f "$INTEGRATION_SUMMARY_FILE" ]; then
        local pfx ph pubh pb pubb cdp cdc folded_total
        case "$artifact_key" in
            dkg) pfx=".folded_artifacts.dkg_aggregator" ;;
            dec) pfx=".folded_artifacts.decryption_aggregator" ;;
            *) pfx="" ;;
        esac
        if [ -n "$pfx" ]; then
            ph=$(jq -r "${pfx}.proof_hex // empty" "$INTEGRATION_SUMMARY_FILE" 2>/dev/null || true)
            pubh=$(jq -r "${pfx}.public_inputs_hex // empty" "$INTEGRATION_SUMMARY_FILE" 2>/dev/null || true)
            if [ -n "$ph" ] && [ "$ph" != "null" ] && [ -n "$pubh" ] && [ "$pubh" != "null" ]; then
                pb=$(hex_len_bytes "$ph")
                pubb=$(hex_len_bytes "$pubh")
                cdp=$(calldata_gas_from_hex "$ph")
                cdc=$(calldata_gas_from_hex "$pubh")
                folded_calldata=$((cdp + cdc))
                folded_total="N/A"
                if [ "$verify_gas" != "N/A" ]; then folded_total=$((verify_gas + folded_calldata)); fi
                echo "| $name | $(format_kb "$pb") KB | $(format_kb "$pubb") KB | $verify_gas | $folded_calldata | $folded_total |" >> "$OUTPUT_FILE"
                return
            fi
        fi
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

integration_timing_seconds() {
    local label="$1"
    local val=""
    local f
    for f in "$INTEGRATION_SUMMARY_FILE" "$GAS_JSON"; do
        [ -n "$f" ] && [ -f "$f" ] || continue
        val=$(jq -r --arg label "$label" '.integration_summary.timings_seconds[]? | select(.label == $label) | .seconds' "$f" 2>/dev/null | head -1)
        if [ -n "$val" ] && [ "$val" != "null" ]; then
            echo "$val"
            return
        fi
        val=$(jq -r --arg label "$label" '.timings_seconds[]? | select(.label == $label) | .seconds' "$f" 2>/dev/null | head -1)
        if [ -n "$val" ] && [ "$val" != "null" ]; then
            echo "$val"
            return
        fi
    done
    echo ""
}

# Normalized integration summary object: either `results_*/integration_summary.json` or
# `crisp_verify_gas.json` → `.integration_summary` (see `BENCHMARK_SUMMARY_OUTPUT` in e3-tests).
integration_blob_from_inputs() {
    local f blob
    for f in "$INTEGRATION_SUMMARY_FILE" "$GAS_JSON"; do
        [ -n "$f" ] && [ -f "$f" ] || continue
        blob=$(jq -c 'if (.integration_summary != null) and (.integration_summary | type == "object") then .integration_summary elif has("integration_test") then . else empty end' "$f" 2>/dev/null || true)
        if [ -n "$blob" ] && [ "$blob" != "null" ]; then
            echo "$blob"
            return 0
        fi
    done
    return 1
}

artifact_size_pair_from_gas() {
    local key="$1"
    local proof public bandwidth
    if [ -f "$GAS_JSON" ]; then
        proof=$(jq -r ".artifact_sizes_bytes.${key}.proof // empty" "$GAS_JSON" 2>/dev/null)
        public=$(jq -r ".artifact_sizes_bytes.${key}.public_inputs // empty" "$GAS_JSON" 2>/dev/null)
        if [ -n "$proof" ] && [ "$proof" != "null" ] && [ -n "$public" ] && [ "$public" != "null" ] \
            && [ "$proof" != "0" ] && [ "$public" != "0" ]; then
            bandwidth=$(echo "$proof + $public" | bc)
            echo "$(format_kb "$proof") KB|$(format_kb "$bandwidth") KB"
            return
        fi
    fi
    # Fallback: folded hex from integration summary export (test `BENCHMARK_SUMMARY_OUTPUT`).
    if [ -n "$INTEGRATION_SUMMARY_FILE" ] && [ -f "$INTEGRATION_SUMMARY_FILE" ]; then
        local pfx ph pubh
        case "$key" in
            dkg) pfx=".folded_artifacts.dkg_aggregator" ;;
            dec) pfx=".folded_artifacts.decryption_aggregator" ;;
            *) echo ""; return ;;
        esac
        ph=$(jq -r "${pfx}.proof_hex // empty" "$INTEGRATION_SUMMARY_FILE" 2>/dev/null || true)
        pubh=$(jq -r "${pfx}.public_inputs_hex // empty" "$INTEGRATION_SUMMARY_FILE" 2>/dev/null || true)
        if [ -n "$ph" ] && [ "$ph" != "null" ] && [ -n "$pubh" ] && [ "$pubh" != "null" ]; then
            proof=$(hex_len_bytes "$ph")
            public=$(hex_len_bytes "$pubh")
            if [ "$proof" != "0" ] || [ "$public" != "0" ]; then
                bandwidth=$(echo "$proof + $public" | bc)
                echo "$(format_kb "$proof") KB|$(format_kb "$bandwidth") KB"
                return
            fi
        fi
    fi
    echo ""
}

load_protocol_params() {
    local default_mod="${REPO_ROOT}/circuits/lib/src/configs/default/mod.nr"
    local committee_name
    committee_name=$(python3 - "$default_mod" <<'PY'
import re, sys
p = sys.argv[1]
try:
    txt = open(p, "r", encoding="utf-8").read()
except Exception:
    print("")
    raise SystemExit(0)
m = re.search(r"committee::([a-zA-Z0-9_]+)::\{H,\s*N_PARTIES,\s*T\}", txt)
print(m.group(1) if m else "")
PY
)
    [ -z "$committee_name" ] && committee_name="micro"
    local committee_file="${REPO_ROOT}/circuits/lib/src/configs/committee/${committee_name}.nr"
    local n t h
    n=$(rg -N "N_PARTIES: u32 = " "$committee_file" | sed -E 's/.*= ([0-9]+);/\1/' | head -1)
    t=$(rg -N "T: u32 = " "$committee_file" | sed -E 's/.*= ([0-9]+);/\1/' | head -1)
    h=$(rg -N "H: u32 = " "$committee_file" | sed -E 's/.*= ([0-9]+);/\1/' | head -1)
    [ -z "$n" ] && n="N/A"
    [ -z "$t" ] && t="N/A"
    [ -z "$h" ] && h="N/A"
    echo "$h|$n|$t"
}

TIMESTAMP=$(date -u "+%Y-%m-%d %H:%M:%S UTC")
IFS='|' read -r PROTOCOL_H PROTOCOL_N PROTOCOL_T <<< "$(load_protocol_params)"

cat > "$OUTPUT_FILE" <<EOF
# Enclave ZK Circuit Benchmarks

**Generated:** ${TIMESTAMP}

**Git Branch:** \`${GIT_BRANCH}\`  
**Git Commit:** \`${GIT_COMMIT}\`

**Committee Size:** \`H=${PROTOCOL_H}\`, \`N=${PROTOCOL_N}\`, \`T=${PROTOCOL_T}\`

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

p1=$(sum_phase_metrics "/dkg/pk /threshold/pk_generation /dkg/sk_share_computation /dkg/e_sm_share_computation /dkg/share_encryption /dkg/share_encryption /dkg/share_decryption /dkg/share_decryption /recursive_aggregation/c2ab_fold /recursive_aggregation/c3ab_fold /recursive_aggregation/c4ab_fold /recursive_aggregation/node_fold")
p2=$(sum_phase_metrics "/threshold/pk_aggregation")
p3=$(sum_phase_metrics "/threshold/user_data_encryption_ct0 /threshold/user_data_encryption_ct1")
p4n=$(sum_phase_metrics "/threshold/share_decryption")
p4a=$(sum_phase_metrics "/threshold/decrypted_shares_aggregation")
IFS='|' read -r p1t p1s p1b <<< "$p1"
IFS='|' read -r p2t p2s p2b <<< "$p2"
IFS='|' read -r p3t p3s p3b <<< "$p3"
IFS='|' read -r p4nt p4ns p4nb <<< "$p4n"
IFS='|' read -r p4at p4as p4ab <<< "$p4a"

# Prefer integration-run timings for phase rows that include fold/consistency work.
p1_integration=$(integration_timing_seconds "ThresholdShares -> PublicKeyAggregated")
if [ -n "$p1_integration" ] && [ "$p1_integration" != "null" ]; then
    p1t="$(format_s "$p1_integration") s"
fi
p4a_integration=$(integration_timing_seconds "Ciphertext published -> PlaintextAggregated")
if [ -n "$p4a_integration" ] && [ "$p4a_integration" != "null" ]; then
    p4at="$(format_s "$p4a_integration") s"
fi

# Keep role-phase rows aligned with artifact outputs when folded artifact sizes are available.
p2_artifact=$(artifact_size_pair_from_gas "dkg")
if [ -n "$p2_artifact" ]; then
    IFS='|' read -r p2s p2b <<< "$p2_artifact"
fi
p3_artifact_proof=""
wrapper_json=$(find_json_by_path_fragment "/threshold/user_data_encryption")
if [ -n "$wrapper_json" ]; then
    p3_proof_bytes=$(jq -r '.proof_generation.proof_size_bytes // 0' "$wrapper_json")
    p3_public_bytes=$(jq -r '.verification.public_inputs_size_bytes // 0' "$wrapper_json")
    p3_bandwidth_bytes=$(echo "$p3_proof_bytes + $p3_public_bytes" | bc)
    p3s="$(format_kb "$p3_proof_bytes") KB"
    p3b="$(format_kb "$p3_bandwidth_bytes") KB"
fi
p4a_artifact=$(artifact_size_pair_from_gas "dec")
if [ -n "$p4a_artifact" ]; then
    IFS='|' read -r p4as p4ab <<< "$p4a_artifact"
fi

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

INTEGRATION_BLOB="$(integration_blob_from_inputs || true)"
if [ -n "$INTEGRATION_BLOB" ]; then
    it_name=$(jq -r '.integration_test // "test_trbfv_actor"' <<<"$INTEGRATION_BLOB")
    {
        echo ""
        echo "## Integration test (\`$it_name\`)"
        echo ""
        echo "### End-to-end phase timings (wall clock)"
        echo ""
        echo "| Phase | Duration (s) |"
        echo "|-------|---------------|"
    } >> "$OUTPUT_FILE"
    while IFS=$'\t' read -r label sec; do
        [ -z "$label" ] && continue
        echo "| $label | $(format_s "$sec") |" >> "$OUTPUT_FILE"
    done < <(jq -r '.timings_seconds[]? | [.label, .seconds] | @tsv' <<<"$INTEGRATION_BLOB")

    if jq -e '.multithread != null' <<<"$INTEGRATION_BLOB" >/dev/null 2>&1; then
        rt=$(jq -r '.multithread.rayon_threads' <<<"$INTEGRATION_BLOB")
        mx=$(jq -r '.multithread.max_simultaneous_rayon_tasks' <<<"$INTEGRATION_BLOB")
        cr=$(jq -r '.multithread.cores_available' <<<"$INTEGRATION_BLOB")
        {
            echo ""
            echo "### Thread pool (same process as integration test)"
            echo ""
            echo "| Setting | Value |"
            echo "|---------|-------|"
            echo "| Rayon threads | $rt |"
            echo "| Max simultaneous Rayon tasks | $mx |"
            echo "| Cores available | $cr |"
        } >> "$OUTPUT_FILE"
    fi

    if jq -e '(.operation_timings | type == "array") and (.operation_timings | length > 0)' <<<"$INTEGRATION_BLOB" >/dev/null 2>&1; then
        {
            echo ""
            echo "### CPU-bound operation timings (tracked in-process)"
            echo ""
            echo "| Name | Avg (s) | Runs | Total (s) |"
            echo "|------|---------|------|-----------|"
        } >> "$OUTPUT_FILE"
        while IFS=$'\t' read -r name avgr runs tot; do
            [ -z "$name" ] && continue
            echo "| $name | $(format_s "$avgr") | $runs | $(format_s "$tot") |" >> "$OUTPUT_FILE"
        done < <(jq -r '.operation_timings[]? | [.name, .avg_seconds, .runs, .total_seconds] | @tsv' <<<"$INTEGRATION_BLOB")
        ott=$(jq -r '.operation_timings_total_seconds // empty' <<<"$INTEGRATION_BLOB")
        if [ -n "$ott" ] && [ "$ott" != "null" ]; then
            echo "" >> "$OUTPUT_FILE"
            echo "Sum of tracked operation wall time: **$(format_s "$ott") s** (often much larger than end-to-end wall clock because work runs in parallel)." >> "$OUTPUT_FILE"
        fi
    fi
fi

{
    echo ""
    echo "## Raw circuit benchmark JSON (Nargo)"
    echo ""
} >> "$OUTPUT_FILE"
shopt -s nullglob
raw_files=("$INPUT_DIR"/*.json)
if [ ${#raw_files[@]} -eq 0 ]; then
    echo "_No \`.json\` files in this input directory._" >> "$OUTPUT_FILE"
else
    echo "Source files for the **Circuit Benchmarks** table. Persist this directory with \`crisp_verify_gas.json\` (and optional \`integration_summary.json\`) to regenerate the report without re-running the integration test." >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    echo "| File |" >> "$OUTPUT_FILE"
    echo "|------|" >> "$OUTPUT_FILE"
    for jf in "${raw_files[@]}"; do
        echo "| \`$(basename "$jf")\` |" >> "$OUTPUT_FILE"
    done
fi
shopt -u nullglob

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

cat >> "$OUTPUT_FILE" <<EOF

## Notes

- All nodes are executed on the same machine in this benchmark run, so inter-node network latency is effectively 0.
EOF

echo "✓ Report generated successfully: $OUTPUT_FILE"
