#!/usr/bin/env bash

TIMESTAMP=$(date +%s)
RUN_FILE="./tmp.docker-compose.${TIMESTAMP}.yml"
TEMPLATE_FILE="./docker-compose.yml"

wait_ready() {
    local STACK_NAME="$1"
    until [ "$(docker stack services $STACK_NAME --format '{{.Replicas}}' | awk -F'/' '$1 != $2')" = "" ]; do
        printf "."
        sleep 1
    done
    echo -ne "\r\033[K"
    echo "Stack $STACK_NAME is ready!"
}

wait_removed() {
  local STACK_NAME="$1"
  while docker stack ps $STACK_NAME >/dev/null 2>&1; do
      printf "."
      sleep 1
  done
  echo -ne "\r\033[K"
  echo "Stack $STACK_NAME is removed"
}

OTEL_ENDPOINT=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --otel-endpoint)
            OTEL_ENDPOINT="$2"
            shift 2
            ;;
        *)
            if [ -z "$STACK_NAME" ]; then
                STACK_NAME="$1"
            elif [ -z "$IMAGE_NAME" ]; then
                IMAGE_NAME="$1"
            else
                echo "Error: Unknown argument: $1"
                echo "Usage: $0 <stack-name> <image-name> [--otel-endpoint <endpoint>]"
                exit 1
            fi
            shift
            ;;
    esac
done

if [ -z "$STACK_NAME" ]; then
    echo "Error: Please provide a stack name as an argument"
    echo "Usage: $0 <stack-name> <image-name> [--otel-endpoint <endpoint>]"
    exit 1
fi

if [ -z "$IMAGE_NAME" ]; then
    echo "Error: Please provide an image name as an argument"
    echo "Usage: $0 <stack-name> <image-name> [--otel-endpoint <endpoint>]"
    exit 1
fi

if [ ! -f "$TEMPLATE_FILE" ]; then
    echo "Error: $TEMPLATE_FILE not found"
    exit 1
fi

sed "s|{{IMAGE}}|$IMAGE_NAME|g" $TEMPLATE_FILE > "${RUN_FILE}"

COMPOSE_FILES="-c $RUN_FILE"
if [ -n "$OTEL_ENDPOINT" ] && [[ "$OTEL_ENDPOINT" == *"otel-collector"* ]]; then
    echo "OTEL enabled with internal collector"
    COMPOSE_FILES="$COMPOSE_FILES -c docker-compose.otel.yml"
elif [ -n "$OTEL_ENDPOINT" ]; then
    echo "OTEL enabled with external endpoint: $OTEL_ENDPOINT"
else
    echo "OTEL disabled"
fi

docker stack rm $STACK_NAME
wait_removed $STACK_NAME
docker stack deploy $COMPOSE_FILES $STACK_NAME
wait_ready $STACK_NAME
rm ./tmp.*.*

echo "✅ Stack '$STACK_NAME' deployed successfully!"
