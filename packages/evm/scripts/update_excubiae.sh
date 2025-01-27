#!/usr/bin/env bash

# Smart Contract Synchronization from Excubiae
#
# This script provides a reliable way to keep our contracts up-to-date with the
# excubiae source repository. It performs an automated sync by:
#
# 1. Performing a sparse checkout of the latest contracts
# 2. Copying them to our local contracts directory
#
# Why This Approach?
#
# We chose this method because:
#
# * The NPM package is outdated and doesn't reflect the latest changes
# * Since we use Hardhat instead of Foundry, we can't use Foundry's library management
# * Git submodules can easily become out of sync and add complexity
# * Git subtrees would require maintaining all the code within the excubiae monorepo
#
# This ensures we have the most recent version while keeping our dependency
# management simple and maintainable.

export REPO_URL="https://github.com/privacy-scaling-explorations/excubiae.git"
export BRANCH_NAME="main"
export TEMP_DIR="/tmp/repo-$(date +%s)"
export SOURCE_FOLDER="${TEMP_DIR}/packages/contracts/contracts/src"
export DESTINATION_FOLDER="${PWD}/contracts/excubiae"

cleanup() {
    echo "Cleaning up temporary directory..."
    rm -rf "$TEMP_DIR"
}

set -e
trap cleanup EXIT

echo "Creating temporary directory..."
mkdir -p "$TEMP_DIR"

echo "Cloning repository (sparse checkout)..."
git clone --depth 1 --branch "${BRANCH_NAME}" "${REPO_URL}" "${TEMP_DIR}"

COMMIT_HASH=$(cd "${TEMP_DIR}" && git rev-parse HEAD)

mkdir -p "$DESTINATION_FOLDER"

echo "Copying $SOURCE_FOLDER to $DESTINATION_FOLDER"
if [ -d "$SOURCE_FOLDER" ]; then
  rsync -av --exclude 'test/' "${SOURCE_FOLDER}/" "$DESTINATION_FOLDER"
  find "$DESTINATION_FOLDER" -type f -name "*.sol" -exec sed -i '1{/SPDX-License-Identifier/!i\// SPDX-License-Identifier: MIT
  };/SPDX-License-Identifier/a\//  Copyright (C) 2024 Privacy & Scaling Explorations\n//  Auto-generated from '"${REPO_URL}"'@'"${COMMIT_HASH}"  {} \;
    echo "Copy completed successfully"
else
    echo "Error: Source folder $SOURCE_FOLDER not found"
    exit 1
fi

cat << EOF > ${DESTINATION_FOLDER}/AUTOGENERATED_FOLDER.md
NOTE: This folder is autogenerated and synced from ${REPO_URL}
Copyright (C) 2024 Privacy & Scaling Explorations
EOF

echo "Operation completed successfully"

