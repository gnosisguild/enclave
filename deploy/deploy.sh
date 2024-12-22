#!/usr/bin/env bash

TIMESTAMP=$(date +%s)
RUN_FILE="./deploy/tmp.docker-compose.${TIMESTAMP}.yml"
TEMPLATE_FILE="./deploy/docker-compose.yml"

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


if [ -z "$1" ]; then
    echo "Error: Please provide a stack name as an argument"
    echo "Usage: $0 <stack-name> <image-name>"
    exit 1
fi

if [ -z "$2" ]; then
    echo "Error: Please provide an image name as an argument"
    echo "Usage: $0 <stack-name> <image-name>"
    exit 1
fi

# Check if docker-compose.yml exists
if [ ! -f "$TEMPLATE_FILE" ]; then
    echo "Error: $TEMPLATE_FILE not found"
    exit 1
fi

sed "s|{{IMAGE}}|$2|g" $TEMPLATE_FILE > "${RUN_FILE}"

cat $RUN_FILE

STACK_NAME=$1
docker stack rm $STACK_NAME
wait_removed $STACK_NAME
docker stack deploy -c $RUN_FILE $STACK_NAME
wait_ready $STACK_NAME
rm ./deploy/tmp.*.*
