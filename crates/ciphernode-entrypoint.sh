#!/bin/bash
set -e

CONFIG_FILE="$CONFIG_DIR/config.yaml"
SECRETS_FILE="/run/secrets/secrets.json"
AGGREGATOR="$AGGREGATOR"

if [ ! -f "$CONFIG_FILE" ]; then
    echo "Error: Config file $CONFIG_FILE not found!"
    exit 1
fi

if [ ! -f "$SECRETS_FILE" ]; then
    echo "Error: Secrets file $SECRETS_FILE not found!"
    exit 1
fi

PRIVATE_KEY=$(jq -r '.private_key' "$SECRETS_FILE")
PASSWORD=$(jq -r '.password' "$SECRETS_FILE")
NETWORK_PRIVATE_KEY=$(jq -r '.network_private_key' "$SECRETS_FILE")

if [ -z "$PRIVATE_KEY" ] || [ -z "$PASSWORD" ] || [ -z "$NETWORK_PRIVATE_KEY" ]; then
    echo "Error: Missing 'private_key', 'password' or 'network_private_key' in secrets file!"
    exit 1
fi

echo "Setting password"
enclave password set --config "$CONFIG_FILE" --password "$PASSWORD"

echo "Setting network private key"
enclave net set-key --config "$CONFIG_FILE" --net-keypair "$NETWORK_PRIVATE_KEY"

OTEL_ARG=""
if [ -n "$OTEL_EXPORTER_OTLP_ENDPOINT" ]; then
    OTEL_ARG="--otel $OTEL_EXPORTER_OTLP_ENDPOINT"
    echo "OTEL telemetry enabled: $OTEL_EXPORTER_OTLP_ENDPOINT"
fi

if [ -n "$OTEL_SERVICE_NAME" ]; then
    echo "Service name for telemetry: $OTEL_SERVICE_NAME"
fi

if [ "$AGGREGATOR" = "true" ]; then
    echo "Setting private key"
    enclave wallet set --config "$CONFIG_FILE" --private-key "$PRIVATE_KEY"

    echo "Starting aggregator"
    exec enclave start -v --config "$CONFIG_FILE" $OTEL_ARG
else
    echo "Starting Ciphernode"
    exec enclave start -v --config "$CONFIG_FILE" $OTEL_ARG
fi


