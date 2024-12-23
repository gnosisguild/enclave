#!/usr/bin/env bash

set_network_private_key() {
    echo "Setting network private key for $1"
    jq --arg key "$2" '.network_private_key = $key' "$1.secrets.json" > "$1.secrets.json.tmp" && mv "$1.secrets.json.tmp" "$1.secrets.json"
}

# Set working directory to script location
cd "$(dirname "$0")" || exit 1

# Source file path (in current directory)
SOURCE="example.secrets.json"

# Color codes
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# List of target files
TARGETS=("cn1" "cn2" "cn3" "agg")

# Sample network private keys
NETWORK_KEY_CN1="0x11a1e500a548b70d88184a1e042900c0ed6c57f8710bcc35dc8c85fa33d3f580"
NETWORK_KEY_CN2="0x21a1e500a548b70d88184a1e042900c0ed6c57f8710bcc35dc8c85fa33d3f580"
NETWORK_KEY_CN3="0x31a1e500a548b70d88184a1e042900c0ed6c57f8710bcc35dc8c85fa33d3f580"
NETWORK_KEY_AGG="0x41a1e500a548b70d88184a1e042900c0ed6c57f8710bcc35dc8c85fa33d3f580"
NET_KEYS=($NETWORK_KEY_CN1 $NETWORK_KEY_CN2 $NETWORK_KEY_CN3 $NETWORK_KEY_AGG)

# Check if source file exists
if [ ! -f "$SOURCE" ]; then
    echo "Error: Source file $SOURCE not found!"
    exit 1
fi

i=0
# Copy file to each target, skipping if exists
for target in "${TARGETS[@]}"; do
    if [ -f "${target}.secrets.json" ]; then
        echo "Skipping ${target}.secrets.json - file already exists"
    else
        cp "$SOURCE" "${target}.secrets.json"
        set_network_private_key "${target}" "${NET_KEYS[${i:-0}]}"
        ((i++))
        echo "Created ${target}.secrets.json"
    fi
done

echo "Copy operation completed!"

# Check for unchanged files
echo -e "\nChecking for unchanged secret files..."
UNCHANGED_FILES=()

for target in "${TARGETS[@]}"; do
    if [ -f "${target}.secrets.json" ]; then
        if cmp -s "$SOURCE" "${target}.secrets.json"; then
            UNCHANGED_FILES+=("${target}.secrets.json")
        fi
    fi
done

# Display warning if unchanged files found
if [ ${#UNCHANGED_FILES[@]} -gt 0 ]; then
    echo -e "${RED}WARNING: The following files are identical to example.secrets.json:${NC}"
    for file in "${UNCHANGED_FILES[@]}"; do
        echo -e "${YELLOW}==> ${NC}${file}${YELLOW} <==${NC}"
    done
    echo -e "${RED}These files should be modified before use in production!${NC}"
fi
