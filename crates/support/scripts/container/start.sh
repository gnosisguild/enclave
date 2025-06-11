#!/usr/bin/env bash
while [[ $# -gt 0 ]]; do
    if [[ $1 == --api-key ]]; then
        export BONSAI_API_KEY="$2"
        shift 2
    elif [[ $1 == --api-url ]]; then
        export BONSAI_API_URL="$2"
        shift 2
    else
        break
    fi
done

[[ -z "$BONSAI_API_KEY" ]] && export RISC0_DEV_MODE=1

exec cargo run --bin e3-support-app "$@"
