#!/usr/bin/env bash
clean_folders() {
    local SCRIPT_DIR=$1

    # Delete output artifacts
    rm -rf "$SCRIPT_DIR/output/"*

    # Delete enclave artifacts
    for name in cn1 cn2 cn3 cn4 ag; do
        # List all files and directories except config.yaml, then delete them
        find "$SCRIPT_DIR/lib/$name" -mindepth 1 ! -regex '.*/config\.yaml$' ! -regex '.*/.gitignore$' -exec rm -rf {} +
    done
}

clean_folders $1
