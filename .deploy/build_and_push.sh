#!/usr/bin/env bash

if [ -z "$1" ]; then 
  echo "please provide a unique tag"
  exit 1
fi

./.deploy/build.sh $1
docker push ghcr.io/gnosisguild/ciphernode:$1

COMPOSE_FILE=./.deploy/docker-compose.yml

TMP_FILE=$(mktemp)

sed "s/^\( *\)image:.*/\1image: ghcr.io\/gnosisguild\/ciphernode:$1/" "$COMPOSE_FILE" > "$TMP_FILE"

mv "$TMP_FILE" "$COMPOSE_FILE"

echo "Successfully updated image references in $COMPOSE_FILE"

docker stack deploy -c .deploy/docker-compose.yml enclave-stack
