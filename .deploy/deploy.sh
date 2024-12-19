#!/usr/bin/env bash

wait_ready() {
    local stack_name="$1"
    until [ "$(docker stack services $stack_name --format '{{.Replicas}}' | awk -F'/' '$1 != $2')" = "" ]; do
        printf "."
        sleep 1
    done
    echo -ne "\r\033[K"
    echo "Stack $stack_name is ready!"
}

wait_removed() {
  local stack_name="$1"
  while docker stack ps $stack_name >/dev/null 2>&1; do
      printf "."
      sleep 1
  done
  echo -ne "\r\033[K"
  echo "Stack $stack_name is removed"
}

stack_name=${1:-enclave}
docker stack rm $stack_name
wait_removed $stack_name
docker stack deploy -c docker-compose.yml --prune $stack_name
wait_ready $stack_name
