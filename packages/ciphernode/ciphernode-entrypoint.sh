#!/bin/bash
set -e

# Paths to config and secrets
CONFIG_FILE="$CONFIG_DIR/config.yaml"
SECRETS_FILE="$SECRETS_DIR/secrets.json"
AGGREGATOR="$AGGREGATOR"

# Ensure required files exist
if [ ! -f "$CONFIG_FILE" ]; then
    echo "Error: Config file $CONFIG_FILE not found!"
    exit 1
fi

if [ ! -f "$SECRETS_FILE" ]; then
    echo "Error: Secrets file $SECRETS_FILE not found!"
    exit 1
fi

# Read secrets from the JSON file
PRIVATE_KEY=$(jq -r '.private_key' "$SECRETS_FILE")
PASSWORD=$(jq -r '.password' "$SECRETS_FILE")

if [ -z "$PRIVATE_KEY" ] || [ -z "$PASSWORD" ]; then
    echo "Error: Missing 'private_key' or 'password' in secrets file!"
    exit 1
fi

# Set password and private key
echo "Setting password"
enclave password create --config "$CONFIG_FILE" --password "$PASSWORD"

if [ "$AGGREGATOR" = "true" ]; then
    echo "Setting private key"
    enclave wallet set --config "$CONFIG_FILE" --private-key "$PRIVATE_KEY"

    echo "Starting aggregator"
    exec enclave aggregator start --config "$CONFIG_FILE"
else
    echo "Starting Ciphernode"
    exec enclave start --config "$CONFIG_FILE"
fi


