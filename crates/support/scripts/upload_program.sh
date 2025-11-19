#!/bin/bash
set -e

# Upload PROGRAM_ELF to Pinata and cache the URL
# Run this when your program changes to avoid runtime uploads

# Determine the script's directory and find the support crate
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUPPORT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

PROGRAM_PATH="$SUPPORT_DIR/target/riscv-guest/methods/guests/riscv32im-risc0-zkvm-elf/release/program.bin"
HASH_FILE="./.program_hash"
URL_FILE="./.program_url"

if [ ! -f "$PROGRAM_PATH" ]; then
    echo "Error: Program not found at $PROGRAM_PATH"
    echo "Run: enclave program compile"
    exit 1
fi

if [ -z "$PINATA_JWT" ]; then
    echo "Error: PINATA_JWT environment variable not set"
    exit 1
fi

# Calculate hash
CURRENT_HASH=$(sha256sum "$PROGRAM_PATH" | awk '{print $1}')

# Check if already uploaded
if [ -f "$HASH_FILE" ] && [ -f "$URL_FILE" ]; then
    STORED_HASH=$(cat "$HASH_FILE")
    if [ "$CURRENT_HASH" = "$STORED_HASH" ]; then
        echo "Program unchanged. Existing URL:"
        cat "$URL_FILE"
        exit 0
    fi
fi

echo "Uploading program to Pinata..."

# Upload
RESPONSE=$(curl -s -X POST "https://api.pinata.cloud/pinning/pinFileToIPFS" \
    -H "Authorization: Bearer $PINATA_JWT" \
    -F "file=@$PROGRAM_PATH;filename=program.bin")

# Extract CID
CID=$(echo "$RESPONSE" | grep -o '"IpfsHash":"[^"]*' | cut -d'"' -f4)

if [ -z "$CID" ]; then
    echo "Upload failed:"
    echo "$RESPONSE"
    exit 1
fi

# Save
PROGRAM_URL="https://gateway.pinata.cloud/ipfs/$CID"
echo "$CURRENT_HASH" > "$HASH_FILE"
echo "$PROGRAM_URL" > "$URL_FILE"

echo ""
echo "✅ Upload successful!"
echo ""
echo "Program URL:"
echo "$PROGRAM_URL"
echo ""
echo "The URL has been saved to .program_url"
echo ""
echo "To use this in production, add to your enclave.config.yaml:"
echo ""
echo "program:"
echo "  risc0:"
echo "    risc0_dev_mode: 0"
echo "    boundless:"
echo "      rpc_url: \"https://sepolia.infura.io/v3/YOUR_KEY\""
echo "      private_key: \"\${PRIVATE_KEY}\""
echo "      pinata_jwt: \"\${PINATA_JWT}\""
echo "      program_url: \"$PROGRAM_URL\""
echo "      onchain: true"

