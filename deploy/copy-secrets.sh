#!/usr/bin/env bash

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

# Check if source file exists
if [ ! -f "$SOURCE" ]; then
    echo "Error: Source file $SOURCE not found!"
    exit 1
fi

# Copy file to each target, skipping if exists
for target in "${TARGETS[@]}"; do
    if [ -f "${target}.secrets.json" ]; then
        echo "Skipping ${target}.secrets.json - file already exists"
    else
        cp "$SOURCE" "${target}.secrets.json"
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
